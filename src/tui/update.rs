//! # Update
//!
//! Update layer of the Elm Architecture: every state transition and every
//! side effect lives behind [`apply`], raw key events entering as
//! [`Message::Key`] and dispatched in context by [`translate_key`].
//!
//! All I/O goes through `model.client`: the model is the sole owner of
//! both the UI state and the email client.

use std::{slice, time::Instant};

use anyhow::{Result, bail};
use edtui::{
    EditorMode, EditorState, Index2, Lines,
    actions::{DeleteChar, Execute, InsertChar, OpenSystemEditor},
};
use mail_parser::MessageParser;
use mml::{
    compiler::message::MmlCompileOptions,
    template::{
        MmlTemplateCursor, compose::MmlTemplateComposeBuilder, forward::MmlTemplateForwardBuilder,
        reply::MmlTemplateReplyBuilder,
    },
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::InputRequest;

use crate::{
    contact::{Contact, ContactLookup, ContactTarget},
    email::{
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
    },
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
            if let Err(err) = model.client.ping() {
                log::warn!("Ping failed: {err}");
            }
            model.last_activity = Instant::now();
            None
        }

        Message::TogglePanel => {
            toggle_panel(model);
            None
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
    model.envelope_total = 0;
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
    model.envelope_total = 0;
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
    if model.envelope_page + 1 < model.total_pages() {
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

fn remove_selected_envelope(model: &mut Model) {
    if model.envelope_index < model.envelopes.len() {
        model.envelopes.remove(model.envelope_index);
        if model.envelope_index >= model.envelopes.len() && model.envelope_index > 0 {
            model.envelope_index -= 1;
        }
    }
}

fn flag_selected_envelope(model: &mut Model, flag: Flag) {
    if let Some(envelope) = model.envelopes.get_mut(model.envelope_index) {
        envelope.flags.insert(flag);
    }
}

fn unflag_selected_envelope(model: &mut Model, flag: Flag) {
    if let Some(envelope) = model.envelopes.get_mut(model.envelope_index) {
        envelope.flags.remove(&flag);
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
    let Some(msg) = mail_parser::MessageParser::default().parse(raw_message) else {
        set_status(model, "Error: failed to parse message");
        return;
    };

    // NOTE: the builder does not interpret the source, so the quote is
    // rendered here. mml carries no PGP backend in this build, so an
    // encrypted thread quotes as an unresolved marker.
    let quote = match MmlTemplateReplyBuilder::quote_options().interpret_msg(&msg) {
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
    .build(&msg, &quote);

    match tpl {
        Ok(tpl) => open_editor_with_template(model, &tpl.content, &tpl.cursor),
        Err(err) => set_status(model, format!("Error building reply template: {err}")),
    }
}

fn start_forward(model: &mut Model, raw_message: &[u8]) {
    let Some(msg) = mail_parser::MessageParser::default().parse(raw_message) else {
        set_status(model, "Error: failed to parse message");
        return;
    };

    let quote = match MmlTemplateForwardBuilder::quote_options().interpret_msg(&msg) {
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
    .build(&msg, &quote);

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
    let result = model.client.list_mailboxes(false);
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
/// The total comes from the backend, not from the page, which cannot tell
/// a full page from the end of the mailbox.
fn load_envelopes(model: &mut Model, landing: EnvelopeLanding) {
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let page = Some(model.envelope_page as u32 + 1);
    let page_size = Some(model.envelope_page_size as u32);

    let result = model
        .client
        .list_envelopes(&mailbox, page, page_size, false);
    match result {
        Ok(list) => {
            model.envelopes = list.envelopes;
            model.envelope_total = list.total;
            let last = model.envelopes.len().saturating_sub(1);
            model.envelope_index = match landing {
                EnvelopeLanding::First => 0,
                EnvelopeLanding::Last => last,
                EnvelopeLanding::Index(index) => index.min(last),
            };
            model.envelope_offset = 0;
            model.status_message = None;
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn read_selected(model: &mut Model) {
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let result = model.client.get_message(&mailbox, &envelope.id);
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

    let result = model.client.get_message(&mailbox, &envelope.id);
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

    let result = model.client.get_message(&mailbox, &envelope.id);
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
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let result = model
        .client
        .copy_messages(&mailbox, &target.id, &[&envelope.id]);
    match result {
        Ok(()) => set_status(model, "Copied"),
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_move(model: &mut Model) {
    let target = model
        .filtered_mailboxes()
        .get(model.dialog_index)
        .map(|m| (**m).clone());
    close_dialog(model);
    let Some(target) = target else { return };
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let result = model
        .client
        .move_messages(&mailbox, &target.id, &[&envelope.id]);
    match result {
        Ok(()) => {
            remove_selected_envelope(model);
            set_status(model, "Moved");
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_flag(model: &mut Model, add: bool) {
    let action: FlagAction = model.selected_flag_action();
    close_dialog(model);

    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let flag = action.flag();
    let label = action.label();

    let op = if add { FlagOp::Add } else { FlagOp::Remove };
    let result = model
        .client
        .store_flags(&mailbox, &[&envelope.id], slice::from_ref(&flag), op);

    match result {
        Ok(()) if add => {
            flag_selected_envelope(model, flag);
            set_status(model, format!("Flag {label} added"));
        }
        Ok(()) => {
            unflag_selected_envelope(model, flag);
            set_status(model, format!("Flag {label} removed"));
        }
        Err(e) => set_status(model, format!("Error: {e}")),
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

    let result = model.client.send_message(mime_bytes);
    match result {
        Ok(()) => {
            set_status(model, "Message sent");
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
    // because a draft is unfinished by nature. IMAP APPEND requires CRLF
    // and edtui emits bare \n, so collapse to \n first to avoid doubling
    // an existing \r\n.
    let raw = model
        .compose_content()
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes();

    let result = model
        .client
        .add_message("Drafts", &[Flag::from_iana(IanaFlag::Draft)], raw);
    match result {
        Ok(_) => {
            set_status(model, "Saved to Drafts");
            cancel_compose(model);
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

/// Decodes a raw message into displayable text, plain part before HTML.
pub fn decode_message_body(raw: &[u8]) -> Result<String> {
    let Some(msg) = MessageParser::default().parse(raw) else {
        bail!("Failed to parse message")
    };

    if let Some(text) = msg.body_text(0) {
        Ok(text.to_string())
    } else if let Some(html) = msg.body_html(0) {
        Ok(html2text::from_read(html.as_bytes(), 80)?)
    } else {
        Ok(String::from_utf8_lossy(raw).to_string())
    }
}
