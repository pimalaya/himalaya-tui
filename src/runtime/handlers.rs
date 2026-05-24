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

//! Modal dialog handlers, one per [`Dialog`] variant, plus the read /
//! reply / forward / save-draft / send helpers they dispatch to.

use io_email::{
    client::EmailClientStd,
    flag::{Flag, IanaFlag},
};
use mml::compiler::message::MmlCompilerBuilder;
use ratatui::crossterm::event::KeyCode;

use crate::{
    app::{compose::ComposeAction, dialog::Dialog, envelopes::EnvelopeAction, state::App},
    mime::{decode_message_body, extract_envelope},
};

pub fn handle_envelope_dialog(app: &mut App, key: KeyCode, client: &mut EmailClientStd) {
    match key {
        KeyCode::Down => app.dialog_next(),
        KeyCode::Up => app.dialog_previous(),
        KeyCode::Enter => {
            let action = app.selected_envelope_action();
            app.close_dialog();
            match action {
                EnvelopeAction::Read => handle_read_message(app, client),
                EnvelopeAction::Reply => handle_reply(app, client, false),
                EnvelopeAction::ReplyAll => handle_reply(app, client, true),
                EnvelopeAction::Forward => handle_forward(app, client),
                EnvelopeAction::Copy => app.open_dialog(Dialog::CopyTo),
                EnvelopeAction::Move => app.open_dialog(Dialog::MoveTo),
                EnvelopeAction::AddFlag => app.open_dialog(Dialog::FlagAdd),
                EnvelopeAction::RemoveFlag => app.open_dialog(Dialog::FlagRemove),
            }
        }
        KeyCode::Esc => app.close_dialog(),
        _ => {}
    }
}

fn handle_read_message(app: &mut App, client: &mut EmailClientStd) {
    let Some(envelope) = app.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = app.selected_mailbox.clone() else {
        return;
    };
    app.set_status(format!("Loading message {}...", envelope.id));

    match client.get_message(&mailbox, &envelope.id) {
        Ok(raw) => match decode_message_body(&raw) {
            Ok(content) => app.show_message(content),
            Err(e) => app.set_status(format!("Error: {e}")),
        },
        Err(e) => app.set_status(format!("Error: {e}")),
    }
}

fn handle_reply(app: &mut App, client: &mut EmailClientStd, reply_all: bool) {
    let Some(envelope) = app.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = app.selected_mailbox.clone() else {
        return;
    };
    app.set_status(format!("Loading message {}...", envelope.id));

    match client.get_message(&mailbox, &envelope.id) {
        Ok(raw) => app.start_reply(&raw, reply_all),
        Err(e) => app.set_status(format!("Error: {e}")),
    }
}

fn handle_forward(app: &mut App, client: &mut EmailClientStd) {
    let Some(envelope) = app.selected_envelope().cloned() else {
        return;
    };
    let Some(mailbox) = app.selected_mailbox.clone() else {
        return;
    };
    app.set_status(format!("Loading message {}...", envelope.id));

    match client.get_message(&mailbox, &envelope.id) {
        Ok(raw) => app.start_forward(&raw),
        Err(e) => app.set_status(format!("Error: {e}")),
    }
}

pub fn handle_copy_to_dialog(app: &mut App, key: KeyCode, client: &mut EmailClientStd) {
    match key {
        KeyCode::Down => app.dialog_next(),
        KeyCode::Up => app.dialog_previous(),
        KeyCode::Enter => {
            let target = app.mailboxes.get(app.dialog_index).cloned();
            app.close_dialog();

            let Some(target) = target else { return };
            let Some(envelope) = app.selected_envelope().cloned() else {
                return;
            };
            let Some(mailbox) = app.selected_mailbox.clone() else {
                return;
            };

            app.set_status(format!("Copying to {}...", target.name));
            match client.copy_messages(&mailbox, &target.id, &[&envelope.id]) {
                Ok(()) => app.set_status("Copied"),
                Err(e) => app.set_status(format!("Error: {e}")),
            }
        }
        KeyCode::Esc => app.close_dialog(),
        _ => {}
    }
}

pub fn handle_move_to_dialog(app: &mut App, key: KeyCode, client: &mut EmailClientStd) {
    match key {
        KeyCode::Down => app.dialog_next(),
        KeyCode::Up => app.dialog_previous(),
        KeyCode::Enter => {
            let target = app.mailboxes.get(app.dialog_index).cloned();
            app.close_dialog();

            let Some(target) = target else { return };
            let Some(envelope) = app.selected_envelope().cloned() else {
                return;
            };
            let Some(mailbox) = app.selected_mailbox.clone() else {
                return;
            };

            app.set_status(format!("Moving to {}...", target.name));
            match client.move_messages(&mailbox, &target.id, &[&envelope.id]) {
                Ok(()) => {
                    app.remove_selected_envelope();
                    app.set_status("Moved");
                }
                Err(e) => app.set_status(format!("Error: {e}")),
            }
        }
        KeyCode::Esc => app.close_dialog(),
        _ => {}
    }
}

pub fn handle_flag_dialog(app: &mut App, key: KeyCode, client: &mut EmailClientStd, add: bool) {
    match key {
        KeyCode::Down => app.dialog_next(),
        KeyCode::Up => app.dialog_previous(),
        KeyCode::Enter => {
            let action = app.selected_flag_action();
            app.close_dialog();

            let Some(envelope) = app.selected_envelope().cloned() else {
                return;
            };
            let Some(mailbox) = app.selected_mailbox.clone() else {
                return;
            };

            let flag = action.flag();
            let label = action.label();
            let verb = if add { "Adding" } else { "Removing" };
            app.set_status(format!("{verb} flag {label}..."));

            let result = if add {
                client.add_flags(&mailbox, &[&envelope.id], &[flag.clone()])
            } else {
                client.delete_flags(&mailbox, &[&envelope.id], &[flag.clone()])
            };

            match result {
                Ok(()) if add => {
                    app.flag_selected_envelope(flag);
                    app.set_status(format!("Flag {label} added"));
                }
                Ok(()) => {
                    app.unflag_selected_envelope(flag);
                    app.set_status(format!("Flag {label} removed"));
                }
                Err(e) => app.set_status(format!("Error: {e}")),
            }
        }
        KeyCode::Esc => app.close_dialog(),
        _ => {}
    }
}

pub fn handle_compose_dialog(app: &mut App, key: KeyCode, client: &mut EmailClientStd) {
    match key {
        KeyCode::Down => app.dialog_next(),
        KeyCode::Up => app.dialog_previous(),
        KeyCode::Enter => {
            let action = app.selected_compose_action();
            match action {
                ComposeAction::Send => {
                    let content = app.compose_content();
                    app.set_status("Compiling message...");
                    match MmlCompilerBuilder::new().build(&content) {
                        Ok(compiler) => match compiler.compile() {
                            Ok(result) => match result.into_vec() {
                                Ok(mime_bytes) => send_compiled(app, mime_bytes, client),
                                Err(e) => app.set_status(format!("Error: {e}")),
                            },
                            Err(e) => app.set_status(format!("Compile error: {e}")),
                        },
                        Err(e) => app.set_status(format!("Parse error: {e}")),
                    }
                }
                ComposeAction::Preview => {
                    let content = app.compose_content();
                    match MmlCompilerBuilder::new().build(&content) {
                        Ok(compiler) => match compiler.compile() {
                            Ok(result) => match result.into_string() {
                                Ok(mime) => {
                                    app.close_dialog();
                                    app.preview_compose(mime);
                                }
                                Err(e) => app.set_status(format!("Error: {e}")),
                            },
                            Err(e) => app.set_status(format!("Compile error: {e}")),
                        },
                        Err(e) => app.set_status(format!("Parse error: {e}")),
                    }
                }
                ComposeAction::SaveToDrafts => save_to_drafts(app, client),
                ComposeAction::Cancel => app.cancel_compose(),
            }
        }
        KeyCode::Esc => app.close_dialog(),
        _ => {}
    }
}

fn save_to_drafts(app: &mut App, client: &mut EmailClientStd) {
    let content = app.compose_content();
    let raw = format!(
        "From: \r\nTo: \r\nSubject: Draft\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{content}"
    )
    .into_bytes();

    app.set_status("Saving to Drafts...");

    match client.add_message("Drafts", &[Flag::from_iana(IanaFlag::Draft)], raw) {
        Ok(_) => {
            app.set_status("Saved to Drafts");
            app.cancel_compose();
        }
        Err(e) => app.set_status(format!("Error: {e}")),
    }
}

fn send_compiled(app: &mut App, mime_bytes: Vec<u8>, client: &mut EmailClientStd) {
    let (from, to) = match extract_envelope(&mime_bytes) {
        Ok(env) => env,
        Err(e) => {
            app.set_status(format!("Send error: {e}"));
            return;
        }
    };
    let to_refs: Vec<&str> = to.iter().map(String::as_str).collect();

    app.set_status("Sending message...");
    match client.send_message(mime_bytes, &from, &to_refs) {
        Ok(()) => {
            app.set_status("Message sent");
            app.cancel_compose();
        }
        Err(e) => app.set_status(format!("Send error: {e}")),
    }
}
