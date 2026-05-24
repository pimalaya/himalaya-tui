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

//! Terminal event loop: polls crossterm events, normalises Vim/Emacs
//! navigation aliases onto the universal keys, then dispatches into
//! the dialog and panel handlers in [`super::handlers`]. Also owns
//! the initial mailbox/envelope fetch performed once the terminal is
//! in the alternate screen.

use std::{io, time::Duration};

use anyhow::Result;
use edtui::actions::{Execute, OpenSystemEditor, system_editor};
use io_email::client::EmailClientStd;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    crossterm::{
        event::{self, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
        execute,
    },
};

use crate::{
    app::{
        dialog::Dialog,
        panel::{BottomPanel, Panel},
        state::App,
    },
    runtime::handlers::{
        handle_compose_dialog, handle_copy_to_dialog, handle_envelope_dialog, handle_flag_dialog,
        handle_move_to_dialog,
    },
    ui,
};

const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(60);

pub fn run(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut client: EmailClientStd,
) -> Result<()> {
    // Initial mailbox + envelope fetch already happened in startup
    // (under the spinner), so the very first frame can show real
    // content instead of a "Connecting..." placeholder.
    run_app(terminal, &mut app, &mut client)
}

pub fn load_envelopes(app: &mut App, client: &mut EmailClientStd) {
    let Some(mailbox) = app.selected_mailbox.clone() else {
        return;
    };

    let page = Some(app.envelope_page as u32 + 1);
    let page_size = Some(app.envelope_page_size as u32);

    match client.list_envelopes(&mailbox, page, page_size, false) {
        Ok(envelopes) => {
            // Total isn't returned by the shared API; approximate with
            // the current page length for now.
            let total = envelopes.len() as u32;
            app.set_envelopes(envelopes, total);
        }
        Err(e) => app.set_status(format!("Error: {e}")),
    }
}

/// Maps mode-specific navigation shortcuts onto the universal keys
/// (`Up`, `Down`, `PageUp`, `PageDown`, `Esc`) consumed by the dialog
/// and global event branches. Returns the original event when no
/// translation applies.
///
/// Both Vim and Emacs aliases are always active because they don't
/// overlap: Vim's flavor uses bare letters (`j`/`k`/`q`) and
/// `Ctrl-d`/`Ctrl-u`, while Emacs uses `Ctrl-n`/`Ctrl-p`/`Ctrl-v`/
/// `Ctrl-g` and `Alt-v`. The composer is the only place where the
/// configured flavor still matters, and it bypasses this translation.
///
/// Emacs flavor: `Ctrl-n`/`Ctrl-p` (line nav), `Ctrl-v`/`Alt-v`
/// (page nav), `Ctrl-g` (cancel).
///
/// Vim flavor: `j`/`k` (line nav), `Ctrl-d`/`Ctrl-u` (page nav), `q`
/// (cancel).
fn translate_key(key: event::KeyEvent) -> event::KeyEvent {
    let translated = match key.modifiers {
        KeyModifiers::NONE => match key.code {
            KeyCode::Char('j') => Some(KeyCode::Down),
            KeyCode::Char('k') => Some(KeyCode::Up),
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

    match translated {
        Some(code) => event::KeyEvent::new(code, KeyModifiers::NONE),
        None => key,
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    client: &mut EmailClientStd,
) -> Result<()> {
    while app.running {
        if app.active_panel == Panel::Compose && system_editor::is_pending(&app.editor_state) {
            system_editor::open(&mut app.editor_state, terminal)?;
            execute!(terminal.backend_mut(), EnableMouseCapture)?;
        }

        terminal.draw(|f| ui::layout::render(f, app))?;

        if !event::poll(KEEPALIVE_INTERVAL)? {
            continue;
        }

        if let Event::Key(raw_key) = event::read()? {
            if raw_key.kind != KeyEventKind::Press {
                continue;
            }

            // The composer hands raw keys to edtui (which already
            // knows Vim vs Emacs); every other branch goes through the
            // translation layer so Ctrl-n/Ctrl-p (Emacs) or j/k (Vim)
            // alias the universal arrow/page keys unconditionally.
            let in_composer = app.dialog.is_none() && app.active_panel == Panel::Compose;
            let key = if in_composer {
                raw_key
            } else {
                translate_key(raw_key)
            };

            if let Some(dialog) = app.dialog {
                match dialog {
                    Dialog::Envelope => handle_envelope_dialog(app, key.code, client),
                    Dialog::Compose => handle_compose_dialog(app, key.code, client),
                    Dialog::CopyTo => handle_copy_to_dialog(app, key.code, client),
                    Dialog::MoveTo => handle_move_to_dialog(app, key.code, client),
                    Dialog::FlagAdd => handle_flag_dialog(app, key.code, client, true),
                    Dialog::FlagRemove => handle_flag_dialog(app, key.code, client, false),
                }
                continue;
            }

            if in_composer {
                if key.code == KeyCode::Esc {
                    app.open_dialog(Dialog::Compose);
                    continue;
                }

                if key.code == KeyCode::Char('e') && key.modifiers.contains(KeyModifiers::ALT) {
                    OpenSystemEditor.execute(&mut app.editor_state);
                } else {
                    app.editor_handler.on_key_event(key, &mut app.editor_state);
                }

                if system_editor::is_pending(&app.editor_state) {
                    system_editor::open(&mut app.editor_state, terminal)?;
                    execute!(terminal.backend_mut(), EnableMouseCapture)?;
                }

                continue;
            }

            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                app.start_compose();
                continue;
            }

            match key.code {
                KeyCode::Esc => {
                    if app.bottom_panel == BottomPanel::MessagePreview {
                        app.close_preview();
                    } else if !app.close_current() {
                        app.quit();
                    }
                }
                KeyCode::Tab => app.toggle_panel(),
                KeyCode::Down => app.next_item(),
                KeyCode::Up => app.previous_item(),
                KeyCode::Enter => handle_enter(app, client),
                KeyCode::PageDown => {
                    if app.active_panel == Panel::Envelopes && app.next_envelope_page() {
                        load_envelopes(app, client);
                    }
                }
                KeyCode::PageUp => {
                    if app.active_panel == Panel::Envelopes && app.prev_envelope_page() {
                        load_envelopes(app, client);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_enter(app: &mut App, client: &mut EmailClientStd) {
    match app.active_panel {
        Panel::Mailboxes => {
            app.select_mailbox();
            load_envelopes(app, client);
        }
        Panel::Envelopes => {
            if app.selected_envelope().is_some() {
                app.open_dialog(Dialog::Envelope);
            }
        }
        Panel::Message => app.close_bottom_panel(),
        Panel::Compose => {}
    }
}
