// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026  soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! Update layer of the Elm Architecture: every state transition and
//! every side effect lives behind [`apply`]. Raw key events enter as
//! [`Message::Key`] and are dispatched in context by [`translate_key`].
//! All I/O goes through `model.client`; the model is the sole owner
//! of both UI state and the email client.

use anyhow::{Result, bail};
use edtui::{
    EditorMode, EditorState, Index2, Lines,
    actions::{Execute, OpenSystemEditor},
};
use io_email::{
    flag::{Flag, FlagOp, IanaFlag},
    mailbox::Mailbox,
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

use crate::tui::model::{
    BottomPanel, ComposeAction, Dialog, EnvelopeAction, FlagAction, Message, Model, Panel,
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
        Message::PageDown => {
            if model.active_panel == Panel::Envelopes && next_envelope_page(model) {
                Some(Message::LoadEnvelopes)
            } else {
                None
            }
        }
        Message::PageUp => {
            if model.active_panel == Panel::Envelopes && prev_envelope_page(model) {
                Some(Message::LoadEnvelopes)
            } else {
                None
            }
        }
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
        Message::ReadSelectedMessage => {
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
        Message::CopySelectedToTarget => {
            do_copy(model);
            None
        }
        Message::MoveSelectedToTarget => {
            do_move(model);
            None
        }
        Message::FlagSelected { add } => {
            do_flag(model, add);
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

    // Alias Vim/Emacs keys onto universal arrows so navigation works
    // regardless of the user's flavor.
    let translated = match key.modifiers {
        KeyModifiers::NONE => match key.code {
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

fn next_item(model: &mut Model) {
    match model.active_panel {
        Panel::Mailboxes => {
            if model.mailbox_index + 1 < model.mailboxes.len() {
                model.mailbox_index += 1;
            }
        }
        Panel::Envelopes => {
            if model.envelope_index + 1 < model.envelopes.len() {
                model.envelope_index += 1;
            }
        }
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
    model.envelope_index = 0;
    model.envelope_offset = 0;
    model.envelope_page = 0;
    model.envelope_total = 0;
    model.envelopes.clear();
    close_bottom_panel(model);
    model.active_panel = Panel::Envelopes;
    model.status_message = Some(format!("Loading envelopes from {}…", m.name));
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
}

fn dialog_next(model: &mut Model) {
    let count = model.dialog_item_count();
    if count > 0 {
        model.dialog_index = (model.dialog_index + 1) % count;
    }
}

fn dialog_previous(model: &mut Model) {
    let count = model.dialog_item_count();
    if count > 0 {
        model.dialog_index = model.dialog_index.checked_sub(1).unwrap_or(count - 1);
    }
}

fn dialog_confirm(model: &mut Model) -> Option<Message> {
    match model.dialog? {
        Dialog::Envelope => {
            let action = model.selected_envelope_action();
            close_dialog(model);
            match action {
                EnvelopeAction::Read => Some(Message::ReadSelectedMessage),
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

    let page = Some(model.envelope_page as u32 + 1);
    let page_size = Some(model.envelope_page_size as u32);

    let result = model
        .client
        .list_envelopes(&mailbox, page, page_size, false);
    match result {
        Ok(envelopes) => {
            // NOTE: the shared API does not yet return a total; we
            // approximate with the current page length.
            let total = envelopes.len() as u32;
            model.envelopes = envelopes;
            model.envelope_index = 0;
            model.envelope_offset = 0;
            model.envelope_total = total;
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

fn do_copy(model: &mut Model) {
    let target = model.mailboxes.get(model.dialog_index).cloned();
    close_dialog(model);
    let Some(target) = target else { return };
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    set_status(model, format!("Copying to {}…", target.name));
    let result = model
        .client
        .copy_messages(&mailbox, &target.id, &[&envelope.id]);
    match result {
        Ok(()) => set_status(model, "Copied"),
        Err(e) => set_status(model, format!("Error: {e}")),
    }
}

fn do_move(model: &mut Model) {
    let target = model.mailboxes.get(model.dialog_index).cloned();
    close_dialog(model);
    let Some(target) = target else { return };
    let Some(envelope) = model.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = model.selected_mailbox.clone() else {
        return;
    };

    set_status(model, format!("Moving to {}…", target.name));
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
    let verb = if add { "Adding" } else { "Removing" };
    set_status(model, format!("{verb} flag {label}…"));

    let op = if add { FlagOp::Add } else { FlagOp::Remove };
    let result = model
        .client
        .store_flags(&mailbox, &[&envelope.id], &[flag.clone()], op);

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
