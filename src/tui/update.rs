//! Update layer of the Elm Architecture: every state transition and
//! every side effect lives behind [`apply`]. Raw key events enter as
//! [`Message::Key`] and are dispatched in context by [`translate_key`].
//! All I/O goes through `model.client`; the model is the sole owner
//! of both UI state and the email client.

use std::{slice, time::Instant};

use anyhow::{Result, bail};
use edtui::{
    EditorMode, EditorState, Index2, Lines,
    actions::{Execute, OpenSystemEditor},
};
use mail_parser::MessageParser;
use mml::{
    compiler::message::MmlCompilerBuilder,
    template::{
        compose::TemplateBuilderCompose, forward::TemplateBuilderForward,
        reply::TemplateBuilderReply, types::TemplateCursor,
    },
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::InputRequest;

use crate::{
    email::{
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
    },
    session::{self, AccountSession},
    tui::{
        command::{self, Command, KeyBinding},
        model::{
            BottomPanel, Dialog, FlagAction, MAILBOX_DIALOG_VISIBLE, Message, Model, Panel,
            format_message_count,
        },
        palette::{self, PaletteMessage},
    },
};

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
        Message::Next => {
            next_item(model);
            None
        }
        Message::Previous => {
            previous_item(model);
            None
        }
        // Panel gating happens at dispatch: every producer of the
        // paging messages goes through the registry's `is_available`.
        Message::PageDown => next_envelope_page(model).then_some(Message::LoadEnvelopes),
        Message::PageUp => prev_envelope_page(model).then_some(Message::LoadEnvelopes),
        Message::Enter => match model.active_panel {
            Panel::Mailboxes => {
                select_mailbox(model);
                Some(Message::LoadEnvelopes)
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

        Message::MailboxFilterKey(key) => {
            mailbox_filter_input(model, key);
            None
        }

        Message::Palette(palette_msg) => palette::update(model, palette_msg),

        Message::OpenDialog(dialog) => {
            open_dialog(model, dialog);
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
        Message::LoadEnvelopes => {
            load_envelopes(model);
            None
        }
        Message::ToggleSelectHovered => {
            toggle_select_hovered(model);
            None
        }
        Message::SelectAllVisible => {
            select_all_visible(model);
            None
        }
        Message::InvertSelectionVisible => {
            invert_selection_visible(model);
            None
        }
        Message::ClearSelection => {
            model.selected_envelope_ids.clear();
            None
        }
        Message::ReadSelected => {
            read_selected(model);
            None
        }
        Message::StartReplyToSelected { reply_all } => {
            fetch_for_reply(model, reply_all);
            None
        }
        Message::StartForwardSelected => {
            fetch_for_forward(model);
            None
        }
        Message::CopySelectedTo(target_id) => {
            do_transfer(model, &target_id, false);
            None
        }
        Message::MoveSelectedTo(target_id) => {
            do_transfer(model, &target_id, true);
            None
        }
        Message::FlagSelected { add, action } => {
            do_flag(model, add, action);
            None
        }
        Message::OpenAccountSwitcher => {
            open_account_switcher(model);
            None
        }
        Message::SwitchAccount(account) => {
            switch_account(model, &account);
            None
        }
        Message::SendCompose => {
            do_send(model);
            None
        }
        Message::PreviewCompose => {
            do_preview(model);
            None
        }
        Message::SaveComposeToDrafts => {
            do_save_draft(model);
            None
        }
        Message::CancelCompose => {
            cancel_compose(model);
            None
        }
    }
}

fn translate_key(model: &Model, key: KeyEvent) -> Option<Message> {
    if model.palette.is_some() {
        return Some(Message::Palette(palette::translate_key(key)));
    }

    // Composer owns its keys; only Esc and Alt-e are intercepted. Esc
    // reuses Message::Esc so apply() can dispatch by model state
    // (composer-mode Esc opens the compose dialog instead of quitting).
    if model.dialog.is_none() && model.active_panel == Panel::Compose {
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
        KeyModifiers::NONE if !in_mailbox_dialog => lookup_alias(PLAIN_ALIASES, key.code),
        KeyModifiers::CONTROL => lookup_alias(CTRL_ALIASES, key.code),
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

    let palette_binding = KeyBinding {
        flavor: None,
        code: KeyCode::Char(model.palette_key),
        modifiers: KeyModifiers::NONE,
    };
    if palette_binding.matches(key, model.keybinds) {
        return Some(Message::Palette(PaletteMessage::Open));
    }

    match code {
        KeyCode::Esc => Some(Message::Esc),
        KeyCode::Tab => Some(Message::TogglePanel),
        KeyCode::Down => Some(Message::Next),
        KeyCode::Up => Some(Message::Previous),
        KeyCode::Enter => Some(Message::Enter),
        _ => command::bound(model, key).map(Command::dispatch),
    }
}

/// Flavor-neutral navigation aliases, translated ahead of dispatch.
/// The reserved-key test in `command/tests.rs` reads these same
/// tables, so registry bindings can never shadow an alias unnoticed.
pub(crate) const PLAIN_ALIASES: &[(char, KeyCode)] = &[
    ('j', KeyCode::Down),
    ('n', KeyCode::Down),
    ('k', KeyCode::Up),
    ('p', KeyCode::Up),
    ('q', KeyCode::Esc),
];

pub(crate) const CTRL_ALIASES: &[(char, KeyCode)] = &[
    ('n', KeyCode::Down),
    ('p', KeyCode::Up),
    ('g', KeyCode::Esc),
];

fn lookup_alias(table: &[(char, KeyCode)], code: KeyCode) -> Option<KeyCode> {
    let KeyCode::Char(c) = code else { return None };
    table
        .iter()
        .find(|(alias, _)| *alias == c)
        .map(|(_, target)| *target)
}

/// Shared key map for `tui_input`-backed filter fields (the mailbox
/// picker and the command palette).
pub(crate) fn input_request(key: KeyEvent) -> Option<InputRequest> {
    match (key.code, key.modifiers) {
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
    }
}

fn mailbox_filter_input(model: &mut Model, key: KeyEvent) {
    let Some(req) = input_request(key) else {
        return;
    };
    model.mailbox_filter.handle(req);
    // Filter changed; snap selection to top of the narrowed list.
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
    if model.has_selection() {
        return Some(Message::ClearSelection);
    }
    if !close_current(model) {
        return Some(Message::Quit);
    }
    None
}

fn toggle_select_hovered(model: &mut Model) {
    let Some(id) = model.selected_envelope().map(|e| e.id.clone()) else {
        return;
    };
    if !model.selected_envelope_ids.remove(&id) {
        model.selected_envelope_ids.insert(id);
    }
    advance_envelope_cursor(model);
}

fn advance_envelope_cursor(model: &mut Model) {
    if model.envelope_index + 1 < model.envelopes.len() {
        model.envelope_index += 1;
    }
}

fn select_all_visible(model: &mut Model) {
    let ids = model.envelopes.iter().map(|e| e.id.clone());
    model.selected_envelope_ids.extend(ids);
}

fn invert_selection_visible(model: &mut Model) {
    for envelope in &model.envelopes {
        if !model.selected_envelope_ids.remove(&envelope.id) {
            model.selected_envelope_ids.insert(envelope.id.clone());
        }
    }
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

fn next_item(model: &mut Model) {
    match model.active_panel {
        Panel::Mailboxes => {
            if model.mailbox_index + 1 < model.mailboxes.len() {
                model.mailbox_index += 1;
            }
        }
        Panel::Envelopes => advance_envelope_cursor(model),
        Panel::Message => {
            model.message_scroll = model.message_scroll.saturating_add(1);
        }
        Panel::Compose => {}
    }
}

fn previous_item(model: &mut Model) {
    match model.active_panel {
        Panel::Mailboxes => {
            model.mailbox_index = model.mailbox_index.saturating_sub(1);
        }
        Panel::Envelopes => {
            model.envelope_index = model.envelope_index.saturating_sub(1);
        }
        Panel::Message => {
            model.message_scroll = model.message_scroll.saturating_sub(1);
        }
        Panel::Compose => {}
    }
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
    reset_envelope_paging(model);
    close_bottom_panel(model);
    model.active_panel = Panel::Envelopes;
    model.status_message = Some(format!("Loading envelopes from {}…", m.name));
}

fn unselect_mailbox(model: &mut Model) {
    model.selected_mailbox = None;
    reset_envelope_paging(model);
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

fn reset_envelope_paging(model: &mut Model) {
    model.envelopes.clear();
    model.envelope_index = 0;
    model.envelope_offset = 0;
    model.envelope_page = 0;
    model.envelope_total = None;
    model.selected_envelope_ids.clear();
}

fn next_envelope_page(model: &mut Model) -> bool {
    if model.can_advance_envelope_page() {
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

/// Drops the rows and their selection entries together, keeping the
/// selection a subset of the visible page.
fn remove_envelopes_by_id(model: &mut Model, ids: &[String]) {
    model.envelopes.retain(|e| !ids.contains(&e.id));
    for id in ids {
        model.selected_envelope_ids.remove(id);
    }
    if model.envelope_index >= model.envelopes.len() {
        model.envelope_index = model.envelopes.len().saturating_sub(1);
    }
}

fn set_flag_on_envelopes(model: &mut Model, ids: &[String], flag: Flag, op: FlagOp) {
    for envelope in model.envelopes.iter_mut().filter(|e| ids.contains(&e.id)) {
        match op {
            FlagOp::Add => {
                envelope.flags.insert(flag.clone());
            }
            FlagOp::Remove => {
                envelope.flags.remove(&flag);
            }
        }
    }
}

fn open_dialog(model: &mut Model, dialog: Dialog) {
    model.dialog = Some(dialog);
    model.dialog_index = 0;
}

fn close_dialog(model: &mut Model) {
    model.dialog = None;
    // The filter is dialog-local state; closing the dialog resets it
    // so the next open starts from a clean slate.
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
            let confirmed = selected_command_message(model, Dialog::Envelope);
            close_dialog(model);
            confirmed
        }
        // Stays open so a failed send/preview leaves the menu usable.
        Dialog::Compose => selected_command_message(model, Dialog::Compose),
        Dialog::CopyTo => transfer_target(model).map(Message::CopySelectedTo),
        Dialog::MoveTo => transfer_target(model).map(Message::MoveSelectedTo),
        Dialog::FlagAdd => confirm_flag(model, true),
        Dialog::FlagRemove => confirm_flag(model, false),
        Dialog::SwitchAccount => confirm_switch_account(model),
    }
}

fn confirm_switch_account(model: &mut Model) -> Option<Message> {
    let confirmed = command::switch_account_candidates(model)
        .into_iter()
        .nth(model.dialog_index)
        .map(|candidate| candidate.message);
    close_dialog(model);
    confirmed
}

// The picker dialogs close here, not in the transfer/flag handlers:
// the dialog layer owns its lifecycle, and the handlers are also
// reachable from the palette with no dialog open.
fn transfer_target(model: &mut Model) -> Option<String> {
    let target_id = model
        .filtered_mailboxes()
        .get(model.dialog_index)
        .map(|m| m.id.clone());
    close_dialog(model);
    target_id
}

fn confirm_flag(model: &mut Model, add: bool) -> Option<Message> {
    let action = model.selected_flag_action();
    close_dialog(model);
    Some(Message::FlagSelected { add, action })
}

fn selected_command_message(model: &Model, dialog: Dialog) -> Option<Message> {
    command::for_context(dialog.command_context()?)
        .nth(model.dialog_index)
        .map(Command::dispatch)
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
    let tpl = TemplateBuilderCompose {
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

    let tpl = TemplateBuilderReply {
        from: model.from.clone().unwrap_or_default(),
        from_name: model.from_name.clone(),
        signature: model.signature.clone(),
        reply_all,
        ..Default::default()
    }
    .build(&msg);

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

    let tpl = TemplateBuilderForward {
        from: model.from.clone().unwrap_or_default(),
        from_name: model.from_name.clone(),
        signature: model.signature.clone(),
        ..Default::default()
    }
    .build(&msg);

    match tpl {
        Ok(tpl) => open_editor_with_template(model, &tpl.content, &tpl.cursor),
        Err(err) => set_status(model, format!("Error building forward template: {err}")),
    }
}

fn open_editor_with_template(model: &mut Model, content: &str, cursor: &TemplateCursor) {
    let mut state = EditorState::new(Lines::from(content));
    state.mode = EditorMode::Insert;
    state.cursor = Index2::new(cursor.row.saturating_sub(1), cursor.col);
    model.editor_state = state;
    model.bottom_panel = BottomPanel::Compose;
    model.active_panel = Panel::Compose;
    model.dialog = None;
}

fn cancel_compose(model: &mut Model) {
    model.dialog = None;
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
                Some(Message::LoadEnvelopes)
            }
        }
        Err(err) => {
            set_status(model, format!("Error: {err}"));
            None
        }
    }
}

fn load_envelopes(model: &mut Model) {
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    // A reload replaces the page; a stale selection must never
    // survive it, even when the fetch fails.
    model.selected_envelope_ids.clear();

    let page = Some(model.envelope_page as u32 + 1);
    let page_size = Some(model.envelope_page_size as u32);

    let result = model
        .client
        .list_envelopes(&mailbox, page, page_size, false);
    match result {
        Ok(listing) => {
            model.envelopes = listing.envelopes;
            model.envelope_index = 0;
            model.envelope_offset = 0;
            model.envelope_total = listing.total;
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
    set_status(model, format!("Loading message {}…", envelope.id));

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
    set_status(model, format!("Loading message {}…", envelope.id));

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
    set_status(model, format!("Loading message {}…", envelope.id));

    let result = model.client.get_message(&mailbox, &envelope.id);
    match result {
        Ok(raw) => start_forward(model, &raw),
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_transfer(model: &mut Model, target_id: &str, is_move: bool) {
    let ids = model.bulk_target_ids();
    if ids.is_empty() {
        return;
    }
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };
    let target_name = model
        .mailbox_name(target_id)
        .unwrap_or(target_id)
        .to_string();

    let (ing, ed) = if is_move {
        ("Moving", "Moved")
    } else {
        ("Copying", "Copied")
    };
    let count = format_message_count(ids.len());
    set_status(model, format!("{ing} {count} to {target_name}…"));
    let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let result = if is_move {
        model.client.move_messages(&mailbox, target_id, &id_refs)
    } else {
        model.client.copy_messages(&mailbox, target_id, &id_refs)
    };
    match result {
        Ok(()) => {
            if is_move {
                remove_envelopes_by_id(model, &ids);
            }
            set_status(model, format!("{ed} {count} to {target_name}"));
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_flag(model: &mut Model, add: bool, action: FlagAction) {
    let ids = model.bulk_target_ids();
    if ids.is_empty() {
        return;
    }
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    let flag = action.flag();
    let label = action.label();
    let verb = if add { "Adding" } else { "Removing" };
    set_status(model, format!("{verb} flag {label}…"));

    let op = if add { FlagOp::Add } else { FlagOp::Remove };
    let id_refs: Vec<&str> = ids.iter().map(String::as_str).collect();
    let result = model
        .client
        .store_flags(&mailbox, &id_refs, slice::from_ref(&flag), op);

    match result {
        Ok(()) => {
            set_flag_on_envelopes(model, &ids, flag, op);
            let verb = if add { "added" } else { "removed" };
            let count = format_message_count(ids.len());
            set_status(model, format!("Flag {label} {verb} ({count})"));
        }
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn open_account_switcher(model: &mut Model) {
    let Some(paths) = model.config_source.clone() else {
        return;
    };
    match session::config_account_names(&paths) {
        Ok(names) => {
            model.account_names = names;
            open_dialog(model, Dialog::SwitchAccount);
        }
        Err(err) => set_status(model, format!("Error: {err}")),
    }
}

/// Re-parses the config and reconnects, even for the active account
/// (mid-session config edits are picked up on any switch). The new
/// session is built before the current one is touched, so a failed
/// switch leaves the session intact.
fn switch_account(model: &mut Model, account: &str) {
    if model.composer_active() {
        set_status(
            model,
            "Send, save or cancel the open draft before switching accounts",
        );
        return;
    }
    let Some(paths) = model.config_source.clone() else {
        return;
    };
    match session::open_account_session(&paths, Some(account)) {
        Ok(new_session) => {
            rebuild_session(model, new_session);
            apply_all(model, Some(Message::Initialize));
            if model.status_message.is_none() {
                set_status(model, format!("Switched to {}", model.account_name));
            }
        }
        Err(err) => set_status(model, format!("Error: {err}")),
    }
}

/// Wholesale rebuild around the new session: only session-independent
/// UI settings survive.
fn rebuild_session(model: &mut Model, new_session: AccountSession) {
    let config_source = model.config_source.take();
    *model = Model {
        editor_handler: model.keybinds.editor_handler(),
        keybinds: model.keybinds,
        palette_key: model.palette_key,
        theme: model.theme,
        ..Model::from_session(new_session, config_source)
    };
}

fn do_send(model: &mut Model) {
    let content = model.compose_content();
    set_status(model, "Compiling message…");
    let mime_bytes = match MmlCompilerBuilder::new().build(&content) {
        Ok(compiler) => match compiler.compile() {
            Ok(result) => match result.into_vec() {
                Ok(bytes) => bytes,
                Err(e) => {
                    set_status(model, format!("Error: {e}"));
                    return;
                }
            },
            Err(e) => {
                set_status(model, format!("Compile error: {e}"));
                return;
            }
        },
        Err(e) => {
            set_status(model, format!("Parse error: {e}"));
            return;
        }
    };

    set_status(model, "Sending message…");
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
    let mime = match MmlCompilerBuilder::new().build(&content) {
        Ok(compiler) => match compiler.compile() {
            Ok(result) => match result.into_string() {
                Ok(s) => s,
                Err(e) => {
                    set_status(model, format!("Error: {e}"));
                    return;
                }
            },
            Err(e) => {
                set_status(model, format!("Compile error: {e}"));
                return;
            }
        },
        Err(e) => {
            set_status(model, format!("Parse error: {e}"));
            return;
        }
    };

    close_dialog(model);
    preview_compose(model, mime);
}

fn do_save_draft(model: &mut Model) {
    // Drafts are unfinished by nature, so we save the composer buffer
    // verbatim (raw MML, partial headers). IMAP APPEND requires CRLF
    // line endings; edtui emits bare `\n`. Normalize first to avoid
    // doubling up if a `\r\n` already exists, then re-CRLF.
    let raw = model
        .compose_content()
        .replace("\r\n", "\n")
        .replace('\n', "\r\n")
        .into_bytes();

    set_status(model, "Saving to Drafts…");

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::email::envelope::Envelope;

    fn press(model: &mut Model, code: KeyCode, modifiers: KeyModifiers) {
        apply_all(model, Some(Message::Key(KeyEvent::new(code, modifiers))));
    }

    #[test]
    fn ctrl_c_starts_a_new_draft() {
        let mut model = Model::default();
        press(&mut model, KeyCode::Char('c'), KeyModifiers::CONTROL);

        assert_eq!(model.bottom_panel, BottomPanel::Compose);
        assert_eq!(model.active_panel, Panel::Compose);
    }

    #[test]
    fn reply_key_dispatches_into_the_reply_flow() {
        let mut model = Model::default();
        model.envelopes.push(Envelope::stub());
        model.selected_mailbox = Some("INBOX".into());

        press(&mut model, KeyCode::Char('r'), KeyModifiers::NONE);

        // The backend-less client cannot fetch the message, so landing
        // on the fetch error proves `r` dispatched into the reply flow.
        assert!(
            model
                .status_message
                .as_deref()
                .is_some_and(|s| s.starts_with("Error")),
            "expected a fetch error status, got {:?}",
            model.status_message
        );
    }

    #[test]
    fn copy_message_dispatches_into_the_transfer_flow() {
        let mut model = Model::default();
        model.envelopes.push(Envelope::stub());
        model.selected_mailbox = Some("INBOX".into());

        apply_all(&mut model, Some(Message::CopySelectedTo("Archive".into())));

        // The backend-less client cannot copy, so landing on the copy
        // error proves the message dispatched into the transfer flow.
        assert!(
            model
                .status_message
                .as_deref()
                .is_some_and(|s| s.starts_with("Error")),
            "expected a transfer error status, got {:?}",
            model.status_message
        );
    }

    #[test]
    fn confirming_an_empty_mailbox_dialog_closes_it() {
        let mut model = Model::default();
        apply_all(&mut model, Some(Message::OpenDialog(Dialog::CopyTo)));

        apply_all(&mut model, Some(Message::DialogConfirm));

        assert!(model.dialog.is_none());
        assert!(model.status_message.is_none());
    }

    #[test]
    fn unbound_keys_do_nothing() {
        let mut model = Model::default();
        press(&mut model, KeyCode::Char('z'), KeyModifiers::NONE);

        assert!(model.status_message.is_none());
        assert_eq!(model.bottom_panel, BottomPanel::None);
    }

    fn model_with_envelopes(ids: &[&str]) -> Model {
        Model {
            envelopes: ids.iter().map(|id| Envelope::stub_with_id(id)).collect(),
            active_panel: Panel::Envelopes,
            ..Model::default()
        }
    }

    #[test]
    fn toggle_select_marks_hovered_and_advances() {
        let mut model = model_with_envelopes(&["1", "2"]);
        apply_all(&mut model, Some(Message::ToggleSelectHovered));

        assert!(model.selected_envelope_ids.contains("1"));
        assert_eq!(model.envelope_index, 1);
    }

    #[test]
    fn space_toggles_selection_through_the_registry() {
        let mut model = model_with_envelopes(&["1", "2"]);
        press(&mut model, KeyCode::Char(' '), KeyModifiers::NONE);

        assert!(model.selected_envelope_ids.contains("1"));
        assert_eq!(model.envelope_index, 1);
    }

    #[test]
    fn toggle_select_advances_the_envelope_cursor_from_any_panel() {
        let mut model = model_with_envelopes(&["1", "2"]);
        model.active_panel = Panel::Mailboxes;
        press(&mut model, KeyCode::Char(' '), KeyModifiers::NONE);

        assert!(model.selected_envelope_ids.contains("1"));
        assert_eq!(model.envelope_index, 1);
        assert_eq!(
            model.mailbox_index, 0,
            "the mailbox cursor must not move as a toggle side effect"
        );
    }

    #[test]
    fn ctrl_a_selects_all_and_ctrl_r_inverts_through_the_registry() {
        let mut model = model_with_envelopes(&["1", "2", "3"]);
        press(&mut model, KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(model.selection_count(), 3);

        press(&mut model, KeyCode::Char('r'), KeyModifiers::CONTROL);
        assert!(model.selected_envelope_ids.is_empty());
    }

    #[test]
    fn toggle_select_unmarks_an_already_selected_envelope() {
        let mut model = model_with_envelopes(&["1", "2"]);
        apply_all(&mut model, Some(Message::ToggleSelectHovered));
        model.envelope_index = 0;
        apply_all(&mut model, Some(Message::ToggleSelectHovered));

        assert!(model.selected_envelope_ids.is_empty());
    }

    #[test]
    fn toggle_select_without_envelopes_does_nothing() {
        let mut model = model_with_envelopes(&[]);
        apply_all(&mut model, Some(Message::ToggleSelectHovered));

        assert!(model.selected_envelope_ids.is_empty());
        assert_eq!(model.envelope_index, 0);
    }

    #[test]
    fn select_all_selects_every_visible_envelope() {
        let mut model = model_with_envelopes(&["1", "2", "3"]);
        apply_all(&mut model, Some(Message::SelectAllVisible));

        assert_eq!(model.selection_count(), 3);
    }

    #[test]
    fn invert_selection_flips_membership() {
        let mut model = model_with_envelopes(&["1", "2", "3"]);
        model.selected_envelope_ids.insert("2".into());
        apply_all(&mut model, Some(Message::InvertSelectionVisible));

        assert!(model.selected_envelope_ids.contains("1"));
        assert!(!model.selected_envelope_ids.contains("2"));
        assert!(model.selected_envelope_ids.contains("3"));
    }

    #[test]
    fn esc_clears_selection_before_closing_anything() {
        let mut model = model_with_envelopes(&["1"]);
        model.selected_mailbox = Some("INBOX".into());
        model.selected_envelope_ids.insert("1".into());
        press(&mut model, KeyCode::Esc, KeyModifiers::NONE);

        assert!(model.selected_envelope_ids.is_empty());
        assert!(model.running);
        assert_eq!(
            model.selected_mailbox.as_deref(),
            Some("INBOX"),
            "Esc with a selection must only clear the selection"
        );
    }

    #[test]
    fn envelope_reload_clears_selection() {
        let mut model = model_with_envelopes(&["1"]);
        model.selected_mailbox = Some("INBOX".into());
        model.selected_envelope_ids.insert("1".into());
        apply_all(&mut model, Some(Message::LoadEnvelopes));

        assert!(model.selected_envelope_ids.is_empty());
    }

    #[test]
    fn paging_reset_clears_selection() {
        let mut model = model_with_envelopes(&["1"]);
        model.selected_envelope_ids.insert("1".into());
        reset_envelope_paging(&mut model);

        assert!(model.selected_envelope_ids.is_empty());
    }

    #[test]
    fn removing_moved_ids_drops_their_rows_and_clamps_the_cursor() {
        let mut model = model_with_envelopes(&["1", "2", "3"]);
        model.envelope_index = 2;
        model.selected_envelope_ids.insert("1".into());
        model.selected_envelope_ids.insert("3".into());
        remove_envelopes_by_id(&mut model, &["1".into(), "3".into()]);

        assert_eq!(model.envelopes.len(), 1);
        assert_eq!(model.envelopes[0].id, "2");
        assert_eq!(model.envelope_index, 0);
        assert!(
            model.selected_envelope_ids.is_empty(),
            "removed rows must leave the selection too"
        );
    }

    #[test]
    fn bulk_flagging_updates_every_targeted_row() {
        let mut model = model_with_envelopes(&["1", "2", "3"]);
        let flag = Flag::from_iana(IanaFlag::Seen);
        let targets = ["1".to_string(), "3".to_string()];

        set_flag_on_envelopes(&mut model, &targets, flag.clone(), FlagOp::Add);
        assert!(model.envelopes[0].flags.contains(&flag));
        assert!(!model.envelopes[1].flags.contains(&flag));
        assert!(model.envelopes[2].flags.contains(&flag));

        set_flag_on_envelopes(&mut model, &targets, flag.clone(), FlagOp::Remove);
        assert!(model.envelopes.iter().all(|e| !e.flags.contains(&flag)));
    }

    fn missing_config_path() -> std::path::PathBuf {
        std::env::temp_dir().join("himalaya-tui-missing-config.toml")
    }

    /// A model whose config source points at a file that does not
    /// exist, so any dispatched switch fails deterministically.
    fn switcher_model() -> Model {
        Model {
            account_name: "work".into(),
            config_source: Some(vec![missing_config_path()]),
            account_names: vec!["personal".into(), "work".into()],
            ..Model::default()
        }
    }

    #[test]
    fn switch_is_refused_while_a_draft_is_open() {
        let mut model = switcher_model();
        model.bottom_panel = BottomPanel::Compose;

        apply_all(&mut model, Some(Message::SwitchAccount("personal".into())));

        assert!(
            model
                .status_message
                .as_deref()
                .is_some_and(|s| s.contains("draft")),
            "expected the draft guard status, got {:?}",
            model.status_message
        );
        assert_eq!(model.bottom_panel, BottomPanel::Compose);
        assert_eq!(model.account_name, "work");
    }

    #[test]
    fn failed_switch_keeps_the_current_session() {
        let mut model = switcher_model();

        apply_all(&mut model, Some(Message::SwitchAccount("personal".into())));

        assert!(
            model
                .status_message
                .as_deref()
                .is_some_and(|s| s.starts_with("Error")),
            "expected a switch error status, got {:?}",
            model.status_message
        );
        assert_eq!(model.account_name, "work");
        assert_eq!(
            model.account_names.len(),
            2,
            "the candidate cache must survive a failed switch"
        );
    }

    #[test]
    fn switch_dialog_confirm_dispatches_the_selected_candidate() {
        let mut model = switcher_model();
        open_dialog(&mut model, Dialog::SwitchAccount);
        apply_all(&mut model, Some(Message::DialogNext));

        apply_all(&mut model, Some(Message::DialogConfirm));

        // The missing config makes the dispatched switch fail, which
        // proves confirm resolved the row to SwitchAccount.
        assert!(model.dialog.is_none());
        assert!(
            model
                .status_message
                .as_deref()
                .is_some_and(|s| s.starts_with("Error")),
            "expected a switch error status, got {:?}",
            model.status_message
        );
    }

    #[test]
    fn open_switcher_without_config_source_does_nothing() {
        let mut model = Model::default();

        apply_all(&mut model, Some(Message::OpenAccountSwitcher));

        assert!(model.dialog.is_none());
        assert!(model.status_message.is_none());
    }

    #[cfg(feature = "maildir")]
    #[test]
    fn switch_rebuilds_the_session_and_carries_ui_settings() -> anyhow::Result<()> {
        let base = std::env::temp_dir().join(format!(
            "himalaya-tui-switch-rebuild-test-{}",
            std::process::id()
        ));
        let mail_root = base.join("mail");
        std::fs::create_dir_all(&mail_root)?;
        let config_path = base.join("config.toml");
        std::fs::write(
            &config_path,
            format!(
                "[accounts.first]\ndefault = true\nmaildir.root = \"{root}\"\n\n\
                 [accounts.second]\nmaildir.root = \"{root}\"\n",
                root = mail_root.display()
            ),
        )?;

        let mut model = Model {
            account_name: "first".into(),
            config_source: Some(vec![config_path]),
            palette_key: ';',
            ..Model::default()
        };

        apply_all(&mut model, Some(Message::SwitchAccount("second".into())));

        assert_eq!(model.account_name, "second");
        assert_eq!(
            model.palette_key, ';',
            "UI settings must survive the rebuild"
        );
        assert_eq!(model.account_names, ["first", "second"]);
        assert_eq!(model.status_message.as_deref(), Some("Switched to second"));
        assert!(model.config_source.is_some());

        std::fs::remove_dir_all(&base)?;
        Ok(())
    }

    #[test]
    fn failed_bulk_op_keeps_the_selection_and_reports_the_error() {
        let mut model = model_with_envelopes(&["1", "2"]);
        model.selected_mailbox = Some("INBOX".into());
        model.mailboxes.push(Mailbox {
            id: "Archive".into(),
            name: "Archive".into(),
            unread: None,
            total: None,
        });
        model.selected_envelope_ids.insert("1".into());
        model.selected_envelope_ids.insert("2".into());
        open_dialog(&mut model, Dialog::MoveTo);

        apply_all(&mut model, Some(Message::DialogConfirm));

        // The backend-less client cannot move, so the rows and the
        // selection must survive the failed call untouched.
        assert!(
            model
                .status_message
                .as_deref()
                .is_some_and(|s| s.starts_with("Error")),
            "expected a move error status, got {:?}",
            model.status_message
        );
        assert_eq!(model.envelopes.len(), 2);
        assert_eq!(model.selection_count(), 2);
    }
}
