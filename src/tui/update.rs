//! # Update
//!
//! Update layer of the Elm Architecture: every state transition and every
//! side effect lives behind [`apply`], raw key events entering as
//! [`Message::Key`] and dispatched in context by [`translate_key`].
//!
//! All I/O goes through `model.client`: the model is the sole owner of
//! both the UI state and the email client.

use std::{borrow::Cow, time::Instant};

use anyhow::Result;
use edtui::{
    EditorMode, EditorState, Index2, Lines,
    actions::{DeleteChar, Execute, InsertChar, OpenSystemEditor},
};
use mime::tree::{cst::MimeCst, entity::MimeEntity};
use mml::{
    compiler::message::MmlCompileOptions,
    interpreter::layer::MmlMimeLayer,
    template::{
        MmlTemplateCursor, compose::MmlTemplateComposeBuilder, forward::MmlTemplateForwardBuilder,
        reply::MmlTemplateReplyBuilder,
    },
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::InputRequest;

use crate::{
    client::{DeleteAction, DeleteReceipt, Receipt},
    contact::{Contact, ContactLookup, ContactTarget},
    email::mailbox::Mailbox,
    tui::model::{
        Blocking, BottomPanel, ComposeAction, ContactCompletion, Dialog, EnvelopeAction,
        EnvelopeLanding, FlagAction, MAILBOX_DIALOG_VISIBLE, Message, Model, Panel,
    },
};

/// Applies a message and every message it cascades into.
pub fn apply_all(model: &mut Model, mut next_msg: Option<Message>) {
    while let Some(msg) = next_msg {
        next_msg = apply(model, msg);
    }
}

fn apply(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::Key(key) => translate_key(model, key),

        Message::Quit => {
            model.running = false;
            None
        }
        Message::Initialize => Some(Message::LoadMailboxes),

        Message::Ping => {
            if let Err(err) = model.client.keep_alive() {
                log::warn!("Keepalive failed: {err}");
            }
            model.last_activity = Instant::now();
            None
        }

        Message::TogglePanel => {
            toggle_panel(model);
            None
        }
        Message::ToggleMark => {
            toggle_mark(model);
            next_item(model)
        }
        Message::Next => next_item(model),
        Message::Previous => previous_item(model),
        Message::PageDown => {
            if model.active_panel == Panel::Envelopes && next_envelope_page(model) {
                Some(Message::LoadEnvelopes(EnvelopeLanding::First))
            } else {
                None
            }
        }
        Message::PageUp => {
            if model.active_panel == Panel::Envelopes && prev_envelope_page(model) {
                Some(Message::LoadEnvelopes(EnvelopeLanding::First))
            } else {
                None
            }
        }
        Message::Enter => match model.active_panel {
            Panel::Mailboxes => {
                select_mailbox(model);
                Some(Message::LoadEnvelopes(EnvelopeLanding::First))
            }
            Panel::Envelopes => {
                if model.selected_envelope().is_some() {
                    open_dialog(model, Dialog::Envelope);
                }
                None
            }
            Panel::Message => {
                close_bottom_panel(model);
                None
            }
            Panel::Compose => None,
        },
        Message::Esc => esc_cascade(model),
        Message::StartCompose => {
            start_compose(model);
            None
        }

        Message::EditorKey(key) => {
            model
                .editor_handler
                .on_key_event(key, &mut model.editor_state);
            None
        }
        Message::OpenSystemEditor => {
            OpenSystemEditor.execute(&mut model.editor_state);
            None
        }

        Message::CompleteContact(target) => {
            complete_contact(model, target);
            None
        }
        Message::PollContactLookup => {
            poll_contact_lookup(model);
            None
        }
        Message::ContactCompletionNext => {
            contact_completion_next(model);
            None
        }
        Message::ContactCompletionPrevious => {
            contact_completion_previous(model);
            None
        }
        Message::ContactCompletionAccept => {
            contact_completion_accept(model);
            None
        }
        Message::ContactCompletionDismiss(key) => {
            model.contact_completion = None;
            key.map(Message::Key)
        }

        Message::MailboxFilterKey(key) => {
            mailbox_filter_input(model, key);
            None
        }

        Message::DialogNext => {
            dialog_next(model);
            None
        }
        Message::DialogPrevious => {
            dialog_previous(model);
            None
        }
        Message::DialogClose => {
            close_dialog(model);
            None
        }
        Message::DialogConfirm => dialog_confirm(model),

        Message::LoadMailboxes => load_mailboxes(model),
        Message::LoadEnvelopes(landing) => announce(model, Blocking::LoadEnvelopes(landing)),
        Message::ReadSelected => announce(model, Blocking::Read),
        Message::StartReplyToSelected { reply_all } => {
            announce(model, Blocking::Reply { reply_all })
        }
        Message::StartForwardSelected => announce(model, Blocking::Forward),
        Message::CopySelectedToTarget => announce(model, Blocking::Copy),
        Message::MoveSelectedToTarget => announce(model, Blocking::Move),
        Message::FlagSelected { add } => announce(model, Blocking::Flag { add }),
        Message::DeleteSelected => start_delete(model),
        Message::SendCompose => announce(model, Blocking::Send),
        Message::PreviewCompose => {
            do_preview(model);
            None
        }
        Message::SaveComposeToDrafts => announce(model, Blocking::SaveDraft),
        Message::CancelCompose => {
            cancel_compose(model);
            None
        }
        Message::Run(blocking) => {
            // NOTE: the announcing status was drawn already, and nothing
            // draws while the action blocks, so it goes now: an outcome
            // that sets none must not leave it behind.
            model.status_message = None;

            match blocking {
                Blocking::LoadEnvelopes(landing) => load_envelopes(model, landing),
                Blocking::Read => read_selected(model),
                Blocking::Reply { reply_all } => fetch_for_reply(model, reply_all),
                Blocking::Forward => fetch_for_forward(model),
                Blocking::Copy => do_copy(model),
                Blocking::Move => do_move(model),
                Blocking::Flag { add } => do_flag(model, add),
                Blocking::Delete => do_delete(model),
                Blocking::Send => do_send(model),
                Blocking::SaveDraft => do_save_draft(model),
            }

            None
        }
    }
}

/// Shows the status of a blocking action and defers it to the next
/// frame, the loop being unable to draw while it runs.
fn announce(model: &mut Model, blocking: Blocking) -> Option<Message> {
    let status = match blocking {
        Blocking::LoadEnvelopes(_) => model
            .selected_mailbox_name()
            .map(|name| format!("Loading envelopes from {name}…")),
        Blocking::Read | Blocking::Reply { .. } | Blocking::Forward => model
            .selected_envelope()
            .map(|envelope| format!("Loading message {}…", envelope.id)),
        Blocking::Copy => model
            .filtered_mailboxes()
            .get(model.dialog_index)
            .map(|target| format!("Copying to {}…", target.name)),
        Blocking::Move => model
            .filtered_mailboxes()
            .get(model.dialog_index)
            .map(|target| format!("Moving to {}…", target.name)),
        Blocking::Flag { add } => {
            let verb = if add { "Adding" } else { "Removing" };
            let label = model.selected_flag_action().label();
            Some(format!("{verb} flag {label}…"))
        }
        Blocking::Delete => Some(format!("Deleting {}…", messages(model.action_ids().len()))),
        Blocking::Send => Some(String::from("Sending message…")),
        Blocking::SaveDraft => Some(String::from("Saving to Drafts…")),
    };

    if let Some(status) = status {
        set_status(model, status);
    }

    model.deferred = Some(Message::Run(blocking));
    None
}

fn translate_key(model: &Model, key: KeyEvent) -> Option<Message> {
    // NOTE: Esc reuses Message::Esc instead of a composer-local message
    // so apply can dispatch on model state: in the composer it opens the
    // compose dialog rather than quitting.
    if model.dialog.is_none() && model.active_panel == Panel::Compose {
        if model.contact_completion.is_some() {
            return Some(translate_completion_key(model, key));
        }
        if model.contact_complete_key.matches(&key)
            && model.editor_state.mode == EditorMode::Insert
            && let Some(target) = contact_target(model)
        {
            return Some(Message::CompleteContact(target));
        }
        if key.code == KeyCode::Esc {
            return Some(Message::Esc);
        }
        if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::ALT) {
            return Some(Message::OpenSystemEditor);
        }
        return Some(Message::EditorKey(key));
    }

    let in_mailbox_dialog = matches!(model.dialog, Some(Dialog::CopyTo | Dialog::MoveTo));

    let translated = match key.modifiers {
        KeyModifiers::NONE if !in_mailbox_dialog => match key.code {
            KeyCode::Char('j') | KeyCode::Char('n') => Some(KeyCode::Down),
            KeyCode::Char('k') | KeyCode::Char('p') => Some(KeyCode::Up),
            KeyCode::Char('q') => Some(KeyCode::Esc),
            _ => None,
        },
        KeyModifiers::CONTROL => match key.code {
            KeyCode::Char('n') => Some(KeyCode::Down),
            KeyCode::Char('p') => Some(KeyCode::Up),
            KeyCode::Char('v') => Some(KeyCode::PageDown),
            KeyCode::Char('d') => Some(KeyCode::PageDown),
            KeyCode::Char('u') => Some(KeyCode::PageUp),
            KeyCode::Char('g') => Some(KeyCode::Esc),
            _ => None,
        },
        KeyModifiers::ALT => match key.code {
            KeyCode::Char('v') => Some(KeyCode::PageUp),
            _ => None,
        },
        _ => None,
    };

    let code = translated.unwrap_or(key.code);

    if model.dialog.is_some() {
        return match code {
            KeyCode::Down => Some(Message::DialogNext),
            KeyCode::Up => Some(Message::DialogPrevious),
            KeyCode::Enter => Some(Message::DialogConfirm),
            KeyCode::Esc => Some(Message::DialogClose),
            _ if in_mailbox_dialog => Some(Message::MailboxFilterKey(key)),
            _ => None,
        };
    }

    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Message::StartCompose);
    }

    match code {
        KeyCode::Esc => Some(Message::Esc),
        KeyCode::Tab => Some(Message::TogglePanel),
        KeyCode::Down => Some(Message::Next),
        KeyCode::Up => Some(Message::Previous),
        KeyCode::PageDown => Some(Message::PageDown),
        KeyCode::PageUp => Some(Message::PageUp),
        KeyCode::Enter => Some(Message::Enter),
        KeyCode::Char(' ') if model.active_panel == Panel::Envelopes => Some(Message::ToggleMark),
        KeyCode::Delete if model.active_panel == Panel::Envelopes => Some(Message::DeleteSelected),
        _ => None,
    }
}

/// Keys while the completion list is open: it navigates, accepts or
/// closes, and any other key closes it and reaches the composer.
fn translate_completion_key(model: &Model, key: KeyEvent) -> Message {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    match key.code {
        KeyCode::Down => Message::ContactCompletionNext,
        KeyCode::Char('n') if ctrl => Message::ContactCompletionNext,
        KeyCode::Up => Message::ContactCompletionPrevious,
        KeyCode::Char('p') if ctrl => Message::ContactCompletionPrevious,
        KeyCode::Enter => Message::ContactCompletionAccept,
        KeyCode::Esc => Message::ContactCompletionDismiss(None),
        _ if model.contact_complete_key.matches(&key) => Message::ContactCompletionNext,
        _ => Message::ContactCompletionDismiss(Some(key)),
    }
}

/// The recipient fragment under the composer cursor, if it completes.
fn contact_target(model: &Model) -> Option<ContactTarget> {
    let lines = model.editor_state.lines.to_vecs();
    let cursor = model.editor_state.cursor;
    ContactTarget::locate(&lines, cursor.row, cursor.col)
}

fn complete_contact(model: &mut Model, target: ContactTarget) {
    let Some(command) = &model.contact_command else {
        set_status(
            model,
            "No contact command configured, see `contact-command` in config.sample.toml",
        );
        return;
    };

    let status = format!("Searching contacts for `{}`", target.query);
    model.contact_lookup = Some(ContactLookup::spawn(command, target));
    set_status(model, status);
}

/// Collects the lookup's answer, dropped when the buffer moved on.
fn poll_contact_lookup(model: &mut Model) {
    let Some(lookup) = &model.contact_lookup else {
        return;
    };
    let Some(result) = lookup.try_result() else {
        return;
    };
    let Some(lookup) = model.contact_lookup.take() else {
        return;
    };

    model.status_message = None;

    if contact_target(model).as_ref() != Some(&lookup.target) {
        return;
    }

    let mut contacts = match result {
        Ok(contacts) => contacts,
        Err(err) => return set_status(model, format!("Error: {err}")),
    };

    match contacts.len() {
        0 => set_status(
            model,
            format!("No contact matches `{}`", lookup.target.query),
        ),
        1 => insert_contact(model, &lookup.target, &contacts.remove(0)),
        _ => {
            model.contact_completion = Some(ContactCompletion {
                target: lookup.target,
                contacts,
                index: 0,
            })
        }
    }
}

fn contact_completion_next(model: &mut Model) {
    if let Some(completion) = &mut model.contact_completion {
        completion.index = (completion.index + 1) % completion.contacts.len();
    }
}

fn contact_completion_previous(model: &mut Model) {
    if let Some(completion) = &mut model.contact_completion {
        let len = completion.contacts.len();
        completion.index = (completion.index + len - 1) % len;
    }
}

fn contact_completion_accept(model: &mut Model) {
    let Some(completion) = model.contact_completion.take() else {
        return;
    };

    let contact = &completion.contacts[completion.index];
    insert_contact(model, &completion.target, contact);
}

/// Replaces the fragment with the contact's mailbox.
///
/// Goes through edtui's own actions, so undo sees one change.
fn insert_contact(model: &mut Model, target: &ContactTarget, contact: &Contact) {
    let state = &mut model.editor_state;
    state.cursor = Index2::new(target.row, target.end);

    let len = target.end - target.start;
    if len > 0 {
        state.execute(DeleteChar(len));
    }

    for c in contact.to_string().chars() {
        state.execute(InsertChar(c));
    }
}

fn mailbox_filter_input(model: &mut Model, key: KeyEvent) {
    let req = match (key.code, key.modifiers) {
        (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
            Some(InputRequest::InsertChar(c))
        }
        (KeyCode::Backspace, _) => Some(InputRequest::DeletePrevChar),
        (KeyCode::Delete, _) => Some(InputRequest::DeleteNextChar),
        (KeyCode::Left, _) => Some(InputRequest::GoToPrevChar),
        (KeyCode::Right, _) => Some(InputRequest::GoToNextChar),
        (KeyCode::Home, _) => Some(InputRequest::GoToStart),
        (KeyCode::End, _) => Some(InputRequest::GoToEnd),
        _ => None,
    };
    let Some(req) = req else { return };
    model.mailbox_filter.handle(req);
    model.dialog_index = 0;
}

fn esc_cascade(model: &mut Model) -> Option<Message> {
    if model.active_panel == Panel::Compose && model.dialog.is_none() {
        open_dialog(model, Dialog::Compose);
        return None;
    }
    if model.active_panel == Panel::Envelopes && !model.marked.is_empty() {
        model.marked.clear();
        return None;
    }
    if model.bottom_panel == BottomPanel::MessagePreview {
        close_preview(model);
        return None;
    }
    if !close_current(model) {
        return Some(Message::Quit);
    }
    None
}

fn toggle_panel(model: &mut Model) {
    model.active_panel = match model.active_panel {
        Panel::Mailboxes => Panel::Envelopes,
        Panel::Envelopes => match model.bottom_panel {
            BottomPanel::Message | BottomPanel::MessagePreview => Panel::Message,
            BottomPanel::Compose => Panel::Compose,
            BottomPanel::None => Panel::Mailboxes,
        },
        Panel::Message => Panel::Mailboxes,
        Panel::Compose => Panel::Mailboxes,
    };
}

/// Moves one item down, crossing pages when the envelope list runs out.
///
/// A mailbox reads as one list, whatever the page size cuts it into.
fn next_item(model: &mut Model) -> Option<Message> {
    match model.active_panel {
        Panel::Mailboxes => {
            if model.mailbox_index + 1 < model.mailboxes.len() {
                model.mailbox_index += 1;
            }
        }
        Panel::Envelopes => {
            if model.envelope_index + 1 < model.envelopes.len() {
                model.envelope_index += 1;
            } else if next_envelope_page(model) {
                return Some(Message::LoadEnvelopes(EnvelopeLanding::First));
            }
        }
        Panel::Message => {
            model.message_scroll = model.message_scroll.saturating_add(1);
        }
        Panel::Compose => {}
    }

    None
}

/// Moves one item up, crossing back to the previous page's last envelope.
fn previous_item(model: &mut Model) -> Option<Message> {
    match model.active_panel {
        Panel::Mailboxes => {
            model.mailbox_index = model.mailbox_index.saturating_sub(1);
        }
        Panel::Envelopes => {
            if model.envelope_index > 0 {
                model.envelope_index -= 1;
            } else if prev_envelope_page(model) {
                return Some(Message::LoadEnvelopes(EnvelopeLanding::Last));
            }
        }
        Panel::Message => {
            model.message_scroll = model.message_scroll.saturating_sub(1);
        }
        Panel::Compose => {}
    }

    None
}

fn close_current(model: &mut Model) -> bool {
    match model.active_panel {
        Panel::Message | Panel::Compose => {
            close_bottom_panel(model);
            true
        }
        Panel::Envelopes => {
            if model.bottom_panel != BottomPanel::None {
                close_bottom_panel(model);
            } else {
                unselect_mailbox(model);
            }
            true
        }
        _ => false,
    }
}

fn select_mailbox(model: &mut Model) {
    let Some(m) = model.mailboxes.get(model.mailbox_index).cloned() else {
        return;
    };
    model.selected_mailbox = Some(m.id.clone());
    model.envelope_index = 0;
    model.envelope_offset = 0;
    model.envelope_page = 0;
    model.envelope_total = None;
    model.envelope_has_next = false;
    model.envelope_end = None;
    model.marked.clear();
    model.envelopes.clear();
    close_bottom_panel(model);
    model.active_panel = Panel::Envelopes;
}

fn unselect_mailbox(model: &mut Model) {
    model.selected_mailbox = None;
    model.envelopes.clear();
    model.envelope_index = 0;
    model.envelope_offset = 0;
    model.envelope_page = 0;
    model.envelope_total = None;
    model.envelope_has_next = false;
    model.envelope_end = None;
    model.marked.clear();
    close_bottom_panel(model);
    model.active_panel = Panel::Mailboxes;
}

fn set_mailboxes(model: &mut Model, mailboxes: Vec<Mailbox>) {
    model.mailboxes = mailboxes;
    model.mailbox_index = model
        .mailboxes
        .iter()
        .position(|m| m.name.eq_ignore_ascii_case("inbox"))
        .unwrap_or(0);
    if !model.mailboxes.is_empty() {
        select_mailbox(model);
    }
    model.status_message = None;
}

/// Adopts the row count the last render measured as the page size.
///
/// Returns the load that re-pages the list around the envelope under the
/// cursor, or `None` when the page already matches the screen, which is
/// every frame but the first and those following a resize.
pub fn adopt_envelope_capacity(model: &mut Model) -> Option<Message> {
    let capacity = model.envelope_capacity;

    if capacity == 0 || capacity == model.envelope_page_size || model.selected_mailbox.is_none() {
        return None;
    }

    let selected = model.envelope_page * model.envelope_page_size + model.envelope_index;
    model.envelope_page_size = capacity;
    model.envelope_page = selected / capacity;

    Some(Message::LoadEnvelopes(EnvelopeLanding::Index(
        selected % capacity,
    )))
}

fn next_envelope_page(model: &mut Model) -> bool {
    if model.has_next_page() {
        model.envelope_page += 1;
        true
    } else {
        false
    }
}

fn prev_envelope_page(model: &mut Model) -> bool {
    if model.envelope_page > 0 {
        model.envelope_page -= 1;
        true
    } else {
        false
    }
}

fn open_dialog(model: &mut Model, dialog: Dialog) {
    model.dialog = Some(dialog);
    model.dialog_index = 0;
}

fn close_dialog(model: &mut Model) {
    model.dialog = None;
    model.mailbox_filter.reset();
}

fn dialog_next(model: &mut Model) {
    let count = model.dialog_item_count();
    if count == 0 {
        return;
    }
    if matches!(model.dialog, Some(Dialog::CopyTo | Dialog::MoveTo)) {
        let max = MAILBOX_DIALOG_VISIBLE.min(count) - 1;
        model.dialog_index = (model.dialog_index + 1).min(max);
    } else {
        model.dialog_index = (model.dialog_index + 1) % count;
    }
}

fn dialog_previous(model: &mut Model) {
    let count = model.dialog_item_count();
    if count == 0 {
        return;
    }
    if matches!(model.dialog, Some(Dialog::CopyTo | Dialog::MoveTo)) {
        model.dialog_index = model.dialog_index.saturating_sub(1);
    } else {
        model.dialog_index = model.dialog_index.checked_sub(1).unwrap_or(count - 1);
    }
}

fn dialog_confirm(model: &mut Model) -> Option<Message> {
    match model.dialog? {
        Dialog::Envelope => {
            let action = model.selected_envelope_action();
            close_dialog(model);
            match action {
                EnvelopeAction::Read => Some(Message::ReadSelected),
                EnvelopeAction::Reply => Some(Message::StartReplyToSelected { reply_all: false }),
                EnvelopeAction::ReplyAll => Some(Message::StartReplyToSelected { reply_all: true }),
                EnvelopeAction::Forward => Some(Message::StartForwardSelected),
                EnvelopeAction::Copy => {
                    open_dialog(model, Dialog::CopyTo);
                    None
                }
                EnvelopeAction::Move => {
                    open_dialog(model, Dialog::MoveTo);
                    None
                }
                EnvelopeAction::AddFlag => {
                    open_dialog(model, Dialog::FlagAdd);
                    None
                }
                EnvelopeAction::RemoveFlag => {
                    open_dialog(model, Dialog::FlagRemove);
                    None
                }
                EnvelopeAction::Delete => Some(Message::DeleteSelected),
            }
        }
        Dialog::Compose => {
            let action = model.selected_compose_action();
            match action {
                ComposeAction::Send => Some(Message::SendCompose),
                ComposeAction::Preview => Some(Message::PreviewCompose),
                ComposeAction::SaveToDrafts => Some(Message::SaveComposeToDrafts),
                ComposeAction::Cancel => Some(Message::CancelCompose),
            }
        }
        Dialog::CopyTo => Some(Message::CopySelectedToTarget),
        Dialog::MoveTo => Some(Message::MoveSelectedToTarget),
        Dialog::FlagAdd => Some(Message::FlagSelected { add: true }),
        Dialog::FlagRemove => Some(Message::FlagSelected { add: false }),
        Dialog::ConfirmDelete => {
            let confirmed = model.dialog_index == 0;
            close_dialog(model);

            if confirmed {
                announce(model, Blocking::Delete)
            } else {
                None
            }
        }
    }
}

fn show_message(model: &mut Model, content: String) {
    model.message_content = Some(content);
    model.message_scroll = 0;
    model.bottom_panel = BottomPanel::Message;
    model.active_panel = Panel::Message;
}

fn close_bottom_panel(model: &mut Model) {
    model.bottom_panel = BottomPanel::None;
    model.message_content = None;
    model.dialog = None;
    if model.active_panel == Panel::Message || model.active_panel == Panel::Compose {
        model.active_panel = Panel::Envelopes;
    }
}

fn preview_compose(model: &mut Model, content: String) {
    model.message_content = Some(content);
    model.message_scroll = 0;
    model.bottom_panel = BottomPanel::MessagePreview;
    model.active_panel = Panel::Message;
}

fn close_preview(model: &mut Model) {
    model.message_content = None;
    model.message_scroll = 0;
    model.bottom_panel = BottomPanel::Compose;
    model.active_panel = Panel::Compose;
}

fn set_status(model: &mut Model, msg: impl Into<String>) {
    model.status_message = Some(msg.into());
}

fn start_compose(model: &mut Model) {
    let tpl = MmlTemplateComposeBuilder {
        from: model.from.clone().unwrap_or_default(),
        from_name: model.from_name.clone(),
        signature: model.signature.clone(),
        ..Default::default()
    }
    .build();

    match tpl {
        Ok(tpl) => open_editor_with_template(model, &tpl.content, &tpl.cursor),
        Err(err) => set_status(model, format!("Error building template: {err}")),
    }
}

fn start_reply(model: &mut Model, raw_message: &[u8], reply_all: bool) {
    // NOTE: mml builds its templates from its own parse of the source, so
    // the message goes through mml's parser here, not mime-rs.
    let layer = MmlMimeLayer::parse(Cow::Borrowed(raw_message));
    let Ok(msg) = layer.message() else {
        set_status(model, "Error: failed to parse message");
        return;
    };

    // NOTE: the builder does not interpret the source, so the quote is
    // rendered here. mml carries no PGP backend in this build, so an
    // encrypted thread quotes as an unresolved marker.
    let quote = match MmlTemplateReplyBuilder::quote_options().interpret_msg(msg) {
        Ok(quote) => quote,
        Err(err) => {
            set_status(model, format!("Error rendering the quote: {err}"));
            return;
        }
    };

    let tpl = MmlTemplateReplyBuilder {
        from: model.from.clone().unwrap_or_default(),
        from_name: model.from_name.clone(),
        signature: model.signature.clone(),
        reply_all,
        ..Default::default()
    }
    .build(msg, &quote);

    match tpl {
        Ok(tpl) => open_editor_with_template(model, &tpl.content, &tpl.cursor),
        Err(err) => set_status(model, format!("Error building reply template: {err}")),
    }
}

fn start_forward(model: &mut Model, raw_message: &[u8]) {
    let layer = MmlMimeLayer::parse(Cow::Borrowed(raw_message));
    let Ok(msg) = layer.message() else {
        set_status(model, "Error: failed to parse message");
        return;
    };

    let quote = match MmlTemplateForwardBuilder::quote_options().interpret_msg(msg) {
        Ok(quote) => quote,
        Err(err) => {
            set_status(model, format!("Error rendering the quote: {err}"));
            return;
        }
    };

    let tpl = MmlTemplateForwardBuilder {
        from: model.from.clone().unwrap_or_default(),
        from_name: model.from_name.clone(),
        signature: model.signature.clone(),
        ..Default::default()
    }
    .build(msg, &quote);

    match tpl {
        Ok(tpl) => open_editor_with_template(model, &tpl.content, &tpl.cursor),
        Err(err) => set_status(model, format!("Error building forward template: {err}")),
    }
}

fn open_editor_with_template(model: &mut Model, content: &str, cursor: &MmlTemplateCursor) {
    let mut state = EditorState::new(Lines::from(content));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(cursor.row.saturating_sub(1), cursor.col);
    model.editor_state = state;
    model.contact_lookup = None;
    model.contact_completion = None;
    model.bottom_panel = BottomPanel::Compose;
    model.active_panel = Panel::Compose;
    model.dialog = None;
}

fn cancel_compose(model: &mut Model) {
    model.dialog = None;
    model.contact_lookup = None;
    model.contact_completion = None;
    close_bottom_panel(model);
}

fn load_mailboxes(model: &mut Model) -> Option<Message> {
    let result = model.client.list_mailboxes();
    match result {
        Ok(mailboxes) => {
            let was_empty = mailboxes.is_empty();
            set_mailboxes(model, mailboxes);
            if was_empty {
                None
            } else {
                Some(Message::LoadEnvelopes(EnvelopeLanding::First))
            }
        }
        Err(err) => {
            set_status(model, format!("Error: {err}"));
            None
        }
    }
}

/// Loads the current page of the selected mailbox, selecting `landing`.
///
/// The page count comes from the total himalaya reports, not from the
/// page, which cannot tell a full page from the end of the mailbox. A
/// page past the first coming back empty, which a full last page leads
/// to when no total says where the mailbox ends, or which a move leaves
/// behind, is the end: the page before it is loaded instead and marked
/// the last.
fn load_envelopes(model: &mut Model, mut landing: EnvelopeLanding) {
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };
    let size = model.envelope_page_size;

    loop {
        let page = model.envelope_page + 1;

        match model.client.list_envelopes(&mailbox, page, size) {
            Ok(list) if list.envelopes.is_empty() && model.envelope_page > 0 => {
                model.envelope_end = Some(model.envelope_page);
                model.envelope_page -= 1;
                landing = EnvelopeLanding::Last;
            }
            Ok(list) => {
                model.envelopes = list.envelopes;
                model.envelope_total = list.total;
                model.envelope_has_next = list.next_page.is_some();
                let last = model.envelopes.len().saturating_sub(1);
                model.envelope_index = match landing {
                    EnvelopeLanding::First => 0,
                    EnvelopeLanding::Last => last,
                    EnvelopeLanding::Index(index) => index.min(last),
                };
                model.envelope_offset = 0;
                model.status_message = list.errors.first().map(|err| format!("Error: {err}"));
                return;
            }
            Err(e) => {
                set_status(model, format!("Error: {e}"));
                return;
            }
        }
    }
}

fn read_selected(model: &mut Model) {
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let result = model.client.read_message(&mailbox, &envelope.id);
    match result {
        Ok(raw) => match decode_message_body(&raw) {
            Ok(content) => show_message(model, content),
            Err(e) => set_status(model, format!("Error: {e}")),
        },
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn fetch_for_reply(model: &mut Model, reply_all: bool) {
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let result = model.client.read_message(&mailbox, &envelope.id);
    match result {
        Ok(raw) => start_reply(model, &raw, reply_all),
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn fetch_for_forward(model: &mut Model) {
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let result = model.client.read_message(&mailbox, &envelope.id);
    match result {
        Ok(raw) => start_forward(model, &raw),
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_copy(model: &mut Model) {
    let target = model
        .filtered_mailboxes()
        .get(model.dialog_index)
        .map(|m| (**m).clone());
    close_dialog(model);
    let Some(target) = target else { return };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };
    let ids = model.action_ids();
    if ids.is_empty() {
        return;
    }

    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let result = model.client.copy_messages(&mailbox, &target.id, &refs);
    match result {
        Ok(receipt) => {
            settle_marks(model, &receipt);
            set_status(
                model,
                receipt_status(|n| format!("Copied {n}"), &receipt, ids.len()),
            );
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

/// Moves the marked envelopes, or the focused one, then lists the page
/// again: the move took rows from it, maybe from other pages too.
fn do_move(model: &mut Model) {
    let target = model
        .filtered_mailboxes()
        .get(model.dialog_index)
        .map(|m| (**m).clone());
    close_dialog(model);
    let Some(target) = target else { return };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };
    let ids = model.action_ids();
    if ids.is_empty() {
        return;
    }

    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let result = model.client.move_messages(&mailbox, &target.id, &refs);
    match result {
        Ok(receipt) => {
            settle_marks(model, &receipt);

            if receipt.count > 0 {
                load_envelopes(model, EnvelopeLanding::Index(model.envelope_index));
            }

            set_status(
                model,
                receipt_status(|n| format!("Moved {n}"), &receipt, ids.len()),
            );
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_flag(model: &mut Model, add: bool) {
    let action: FlagAction = model.selected_flag_action();
    close_dialog(model);

    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };
    let ids = model.action_ids();
    if ids.is_empty() {
        return;
    }

    let flag = action.flag();
    let label = action.label();
    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();

    let result = model
        .client
        .flag_messages(&mailbox, action.name(), add, &refs);

    match result {
        Ok(receipt) => {
            let failed = failed_ids(&receipt);

            for envelope in &mut model.envelopes {
                if !ids.contains(&envelope.id) || failed.contains(&envelope.id.as_str()) {
                    continue;
                }

                if add {
                    envelope.flags.insert(flag.clone());
                } else {
                    envelope.flags.remove(&flag);
                }
            }

            settle_marks(model, &receipt);

            let verb = if add { "added to" } else { "removed from" };
            let done = |n| format!("Flag {label} {verb} {n}");
            set_status(model, receipt_status(done, &receipt, ids.len()));
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

/// Starts a delete of the marked envelopes, or the focused one: straight
/// away outside the trash, where it moves them there, after a confirmation
/// in the trash, where it removes them for good.
fn start_delete(model: &mut Model) -> Option<Message> {
    if model.action_ids().is_empty() {
        return None;
    }

    if model.selected_mailbox_role() == Some("trash") {
        open_dialog(model, Dialog::ConfirmDelete);
        return None;
    }

    announce(model, Blocking::Delete)
}

/// Deletes the marked envelopes, or the focused one, trash first, then
/// lists the page again.
fn do_delete(model: &mut Model) {
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };
    let ids = model.action_ids();
    if ids.is_empty() {
        return;
    }

    let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let result = model.client.delete_messages(&mailbox, &refs);
    match result {
        Ok(DeleteReceipt { action, receipt }) => {
            settle_marks(model, &receipt);

            if receipt.count > 0 {
                load_envelopes(model, EnvelopeLanding::Index(model.envelope_index));
            }

            let done = |n| match action {
                DeleteAction::MovedToTrash => format!("Moved {n} to the trash"),
                DeleteAction::Deleted => format!("Deleted {n} for good"),
                DeleteAction::Flagged => {
                    format!("Flagged {n} deleted, the server expunges them later")
                }
            };
            set_status(model, receipt_status(done, &receipt, ids.len()));
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

/// Unmarks the envelopes a command acted on, keeping marked the ones its
/// `errors` name, so a partial action can be retried on what failed.
fn settle_marks(model: &mut Model, receipt: &Receipt) {
    let failed = failed_ids(receipt);
    model.marked.retain(|id| failed.contains(&id.as_str()));
}

/// The ids a receipt's `errors` name.
fn failed_ids(receipt: &Receipt) -> Vec<&str> {
    receipt
        .errors
        .iter()
        .filter_map(|err| err.id.as_deref())
        .collect()
}

/// Toggles the mark of the focused envelope.
fn toggle_mark(model: &mut Model) {
    let Some(id) = model
        .selected_envelope()
        .map(|envelope| envelope.id.clone())
    else {
        return;
    };

    match model.marked.iter().position(|marked| *marked == id) {
        Some(index) => {
            model.marked.remove(index);
        }
        None => model.marked.push(id),
    }
}

fn do_send(model: &mut Model) {
    let content = model.compose_content();
    let mime_bytes = match MmlCompileOptions::default().compile(&content) {
        Ok(bytes) => bytes,
        Err(e) => {
            set_status(model, format!("Compile error: {e}"));
            return;
        }
    };

    let result = model.client.send_message(&mime_bytes);
    match result {
        Ok(receipt) => {
            let status = match (receipt.queued, receipt.saved) {
                (true, _) => "Message queued for sending",
                (false, true) => "Message sent, a copy saved",
                (false, false) => "Message sent",
            };
            set_status(model, status);
            cancel_compose(model);
        }
        Err(e) => set_status(model, format!("Send error: {e}")),
    }
}

fn do_preview(model: &mut Model) {
    let content = model.compose_content();
    let mime = match MmlCompileOptions::default()
        .compile(&content)
        .map_err(anyhow::Error::from)
        .and_then(|mime| Ok(String::from_utf8(mime)?))
    {
        Ok(mime) => mime,
        Err(e) => {
            set_status(model, format!("Compile error: {e}"));
            return;
        }
    };

    close_dialog(model);
    preview_compose(model, mime);
}

fn do_save_draft(model: &mut Model) {
    // NOTE: the buffer is saved verbatim (raw MML, partial headers)
    // because a draft is unfinished by nature; himalaya makes its line
    // endings CRLF.
    let raw = model.compose_content().into_bytes();

    let result = model.client.save_draft(&raw);
    match result {
        Ok(receipt) => {
            set_status(
                model,
                queued(String::from("Saved to drafts"), receipt.queued),
            );
            cancel_compose(model);
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

/// The status line of a write on `asked` messages: `done` phrasing how
/// many it acted on (`1 message`, `2 of 3 messages`), then the first one
/// it could not act on.
fn receipt_status(done: impl Fn(String) -> String, receipt: &Receipt, asked: usize) -> String {
    let Some(err) = receipt.errors.first() else {
        let status = done(messages(receipt.count));
        return queued(status, receipt.queued);
    };

    let status = done(format!("{} of {}", receipt.count, messages(asked)));
    format!("{}, error: {err}", queued(status, receipt.queued))
}

/// `n message(s)`.
fn messages(count: usize) -> String {
    match count {
        1 => String::from("1 message"),
        count => format!("{count} messages"),
    }
}

/// `status`, saying a sync engine applies it later when `queued`.
fn queued(status: String, queued: bool) -> String {
    if queued {
        format!("{status} (queued for the next sync)")
    } else {
        status
    }
}

/// Decodes a raw message into displayable text, plain part before HTML.
///
/// The first part of the RFC 8621 §4.1.4 text body is shown, then the
/// first of the HTML body, and the raw bytes when neither is text. The
/// text body holds an HTML part when the message has no plain one.
pub fn decode_message_body(raw: &[u8]) -> Result<String> {
    let parsed = MimeCst::parse(raw);
    let lists = parsed.root.body_lists();

    for part in [lists.text.first(), lists.html.first()]
        .into_iter()
        .flatten()
    {
        if let Some(text) = part_text(part)? {
            return Ok(text);
        }
    }

    Ok(String::from_utf8_lossy(raw).to_string())
}

/// The text of a body part, HTML rendered as text; `None` for a part
/// that is not text, such as an inline image.
fn part_text(part: &MimeEntity) -> Result<Option<String>> {
    let content_type = part.content_type();
    let html = content_type.is("text", "html");

    if !html && !content_type.is("text", "plain") {
        return Ok(None);
    }

    let Some(text) = part.text() else {
        return Ok(None);
    };

    if html {
        // NOTE: undecorated, no `**bold**` and no link footnotes, as
        // mail-parser's own HTML-to-text rendered an HTML-only body.
        let config = html2text::config::plain_no_decorate();
        Ok(Some(config.string_from_read(text.as_bytes(), 80)?))
    } else {
        Ok(Some(text.into_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use himalaya::backend::Backend;
    use tempfile::TempDir;

    use super::{apply_all, decode_message_body};
    use crate::{
        client::MailClient,
        tui::model::{Blocking, Dialog, Message, Model, Panel},
    };

    /// A text part is transfer-decoded and transcoded from its charset.
    #[test]
    fn a_text_part_is_decoded_from_its_charset() {
        let raw = concat!(
            "From: a@x.org\r\n",
            "Content-Type: text/plain; charset=iso-8859-1\r\n",
            "Content-Transfer-Encoding: quoted-printable\r\n",
            "\r\n",
            "caf=E9 cr=E8me\r\n",
        );

        let body = decode_message_body(raw.as_bytes()).unwrap();

        assert_eq!(body, "café crème\r\n");
    }

    /// A multi-byte charset decodes too, where mail-parser built without
    /// its `full_encoding` feature read the bytes as lossy UTF-8.
    #[test]
    fn a_multi_byte_charset_is_decoded() {
        let mut raw = concat!(
            "From: a@x.org\r\n",
            "Content-Type: text/plain; charset=shift_jis\r\n",
            "\r\n",
        )
        .as_bytes()
        .to_vec();
        raw.extend_from_slice(&[0x93, 0xFA, 0x96, 0x7B]);

        let body = decode_message_body(&raw).unwrap();

        assert_eq!(body, "日本");
    }

    /// Of two alternatives the plain one is shown, the HTML left out.
    #[test]
    fn the_plain_alternative_wins_over_the_html_one() {
        let raw = concat!(
            "From: a@x.org\r\n",
            "Content-Type: multipart/alternative; boundary=b\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/plain; charset=utf-8\r\n",
            "\r\n",
            "plain\r\n",
            "--b\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<p>html</p>\r\n",
            "--b--\r\n",
        );

        let body = decode_message_body(raw.as_bytes()).unwrap();

        assert_eq!(body, "plain");
    }

    /// An HTML-only message is rendered as text, its markup gone and no
    /// emphasis marker added in its place.
    #[test]
    fn an_html_only_message_is_rendered_as_text() {
        let raw = concat!(
            "From: a@x.org\r\n",
            "Content-Type: text/html; charset=utf-8\r\n",
            "\r\n",
            "<p>Hello <b>world</b></p>\r\n",
        );

        let body = decode_message_body(raw.as_bytes()).unwrap();

        assert_eq!(body, "Hello world\n");
    }

    /// A Maildir account whose inbox holds `count` messages, and the
    /// model over it with the first page of `page_size` loaded.
    fn loaded(count: usize, page_size: usize) -> (TempDir, Model) {
        loaded_with(count, page_size, true)
    }

    /// [`loaded`], with a `Trash` folder aliased as the trash or none.
    fn loaded_with(count: usize, page_size: usize, trash: bool) -> (TempDir, Model) {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("mail");

        for folder in [root.clone(), root.join("Archive"), root.join("Trash")] {
            for sub in ["cur", "new", "tmp"] {
                fs::create_dir_all(folder.join(sub)).unwrap();
            }
        }

        for n in 1..=count {
            let message = format!(
                "From: a@localhost\r\nSubject: {n}\r\nDate: Mon, 1 Jan 2024 00:00:{n:02} +0000\r\n\r\nbody\r\n"
            );
            fs::write(root.join("cur").join(format!("{n}:2,")), message).unwrap();
        }

        let config = dir.path().join("config.toml");
        let toml = format!(
            "[accounts.test]\ndefault = true\nmaildir.root = {:?}\n",
            root.display().to_string(),
        );
        let toml = if trash {
            toml + "mailbox.alias.trash = \"Trash\"\n"
        } else {
            toml
        };
        fs::write(&config, toml).unwrap();

        let client = MailClient::new(vec![config], None, Backend::Auto);
        let mut model = Model::for_tests(client, page_size);
        run(&mut model, Message::Initialize);
        model.active_panel = Panel::Envelopes;

        (dir, model)
    }

    /// Applies `msg`, then every action it deferred.
    fn run(model: &mut Model, msg: Message) {
        apply_all(model, Some(msg));

        while let Some(deferred) = model.deferred.take() {
            apply_all(model, Some(deferred));
        }
    }

    /// Moves what an action reaches to `Archive`.
    fn move_to_archive(model: &mut Model) {
        model.dialog = Some(Dialog::MoveTo);
        model.dialog_index = model
            .filtered_mailboxes()
            .iter()
            .position(|mailbox| mailbox.name == "Archive")
            .unwrap();
        run(model, Message::Run(Blocking::Move));
    }

    fn archived(model: &mut Model) -> usize {
        model
            .client
            .list_envelopes("Archive", 1, 50)
            .unwrap()
            .envelopes
            .len()
    }

    #[test]
    fn marks_cross_pages_and_one_move_carries_them_all() {
        let (_dir, mut model) = loaded(3, 2);
        assert_eq!(model.envelopes.len(), 2);

        run(&mut model, Message::ToggleMark);
        run(&mut model, Message::ToggleMark);
        assert_eq!(model.envelope_page, 1, "the second mark moved onto page 2");

        run(&mut model, Message::ToggleMark);
        assert_eq!(model.marked.len(), 3);

        move_to_archive(&mut model);

        assert_eq!(archived(&mut model), 3);
        assert!(model.marked.is_empty());
        assert_eq!(model.status_message.as_deref(), Some("Moved 3 messages"));
    }

    #[test]
    fn a_partial_move_keeps_what_failed_marked() {
        let (dir, mut model) = loaded(2, 10);

        run(&mut model, Message::ToggleMark);
        run(&mut model, Message::ToggleMark);
        let gone = model.marked[1].clone();
        fs::remove_file(dir.path().join("mail/cur").join(format!("{gone}:2,"))).unwrap();

        move_to_archive(&mut model);

        assert_eq!(model.marked, [gone]);
        let status = model.status_message.clone().unwrap();
        assert!(
            status.starts_with("Moved 1 of 2 messages, error: "),
            "{status}"
        );
    }

    #[test]
    fn a_flag_reaches_every_marked_envelope() {
        let (_dir, mut model) = loaded(3, 10);

        run(&mut model, Message::ToggleMark);
        run(&mut model, Message::Next);
        run(&mut model, Message::ToggleMark);

        model.dialog = Some(Dialog::FlagAdd);
        model.dialog_index = 1;
        run(&mut model, Message::Run(Blocking::Flag { add: true }));

        let flagged = model
            .envelopes
            .iter()
            .filter(|envelope| {
                envelope
                    .flags
                    .iter()
                    .any(|f| f.raw().eq_ignore_ascii_case("\\Flagged"))
            })
            .count();
        assert_eq!(flagged, 2);
        assert!(model.marked.is_empty());
    }

    #[test]
    fn esc_clears_the_marks_before_anything_else() {
        let (_dir, mut model) = loaded(2, 10);
        run(&mut model, Message::ToggleMark);

        run(&mut model, Message::Esc);

        assert!(model.marked.is_empty());
        assert!(model.running);
        assert_eq!(model.active_panel, Panel::Envelopes);
    }

    #[test]
    fn another_mailbox_clears_the_marks() {
        let (_dir, mut model) = loaded(2, 10);
        run(&mut model, Message::ToggleMark);

        model.active_panel = Panel::Mailboxes;
        run(&mut model, Message::Enter);

        assert!(model.marked.is_empty());
    }

    fn count(model: &mut Model, mailbox: &str) -> usize {
        model
            .client
            .list_envelopes(mailbox, 1, 50)
            .unwrap()
            .envelopes
            .len()
    }

    /// Opens the mailbox named `name` as the mailbox panel does.
    fn open(model: &mut Model, name: &str) {
        model.mailbox_index = model.mailboxes.iter().position(|m| m.name == name).unwrap();
        model.active_panel = Panel::Mailboxes;
        run(model, Message::Enter);
        model.active_panel = Panel::Envelopes;
    }

    #[test]
    fn a_delete_moves_to_the_trash_without_asking() {
        let (_dir, mut model) = loaded(2, 10);

        run(&mut model, Message::DeleteSelected);

        assert_eq!(model.dialog, None);
        assert_eq!(
            model.status_message.as_deref(),
            Some("Moved 1 message to the trash")
        );
        assert_eq!(model.envelopes.len(), 1);
        assert_eq!(count(&mut model, "Trash"), 1);
    }

    #[test]
    fn a_delete_in_the_trash_asks_then_removes_for_good() {
        let (_dir, mut model) = loaded(2, 10);
        run(&mut model, Message::ToggleMark);
        run(&mut model, Message::ToggleMark);
        run(&mut model, Message::DeleteSelected);
        open(&mut model, "Trash");
        assert_eq!(model.envelopes.len(), 2);

        run(&mut model, Message::DeleteSelected);
        assert_eq!(model.dialog, Some(Dialog::ConfirmDelete));

        model.dialog_index = 0;
        run(&mut model, Message::DialogConfirm);

        assert_eq!(
            model.status_message.as_deref(),
            Some("Deleted 1 message for good")
        );
        assert_eq!(count(&mut model, "Trash"), 1);
    }

    #[test]
    fn declining_a_delete_in_the_trash_leaves_everything() {
        let (_dir, mut model) = loaded(1, 10);
        run(&mut model, Message::DeleteSelected);
        open(&mut model, "Trash");

        run(&mut model, Message::DeleteSelected);
        model.dialog_index = 1;
        run(&mut model, Message::DialogConfirm);

        assert_eq!(model.dialog, None);
        assert_eq!(count(&mut model, "Trash"), 1);
    }

    #[test]
    fn a_delete_without_a_trash_names_the_alias_to_set() {
        let (_dir, mut model) = loaded_with(1, 10, false);

        run(&mut model, Message::DeleteSelected);

        let status = model.status_message.clone().unwrap();
        assert!(status.contains("mailbox.alias.trash"), "{status}");
        assert_eq!(model.envelopes.len(), 1);
    }
}
