use std::{io, path::PathBuf};

use anyhow::Result;
use edtui::{actions::system_editor, EditorEventHandler};
use ratatui::{
    backend::CrosstermBackend,
    crossterm::{
        event::{
            self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
            KeyModifiers,
        },
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    Terminal,
};

use himalaya_tui::app::{App, ComposeAction, Panel};
use himalaya_tui::ui;

#[cfg(feature = "imap")]
use himalaya_tui::imap;

fn main() -> Result<()> {
    let config_paths = get_config_paths();
    let account_name = std::env::args().nth(1);

    let mut app = App::new(&config_paths, account_name.as_deref())?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    #[cfg(feature = "imap")]
    {
        app.set_status("Connecting to IMAP server...");
        terminal.draw(|f| ui::render(f, &mut app))?;

        match imap::fetch_mailboxes(&app.imap_config) {
            Ok(mailboxes) => app.set_mailboxes(mailboxes),
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
    }

    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = result {
        eprintln!("Error: {err:?}");
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    // Use Emacs keybindings for edtui
    let mut editor_handler = EditorEventHandler::emacs_mode();

    while app.running {
        terminal.draw(|f| ui::render(f, app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Handle compose dialog (selectable list)
            if app.show_compose_dialog {
                handle_compose_dialog(app, key.code);
                continue;
            }

            // Handle compose mode (edtui with Ctrl-C Ctrl-C to finish)
            if app.active_panel == Panel::Compose {
                // Check for Ctrl-C
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    if app.ctrl_c_pending {
                        // Second Ctrl-C: finish compose
                        app.ctrl_c_pending = false;
                        app.finish_compose();
                    } else {
                        // First Ctrl-C: mark as pending
                        app.ctrl_c_pending = true;
                        app.set_status("Press Ctrl-C again to finish editing");
                    }
                    continue;
                }

                // Any other key resets Ctrl-C pending
                app.ctrl_c_pending = false;
                app.clear_status();

                // Forward to edtui
                editor_handler.on_key_event(key, &mut app.editor_state);

                // Check if system editor was requested (Alt+e)
                if system_editor::is_pending(&app.editor_state) {
                    system_editor::open(&mut app.editor_state, terminal)?;
                    execute!(
                        terminal.backend_mut(),
                        EnableMouseCapture
                    )?;
                }

                continue;
            }

            // Normal mode key handling
            match key.code {
                KeyCode::Char('q') => {
                    // Close current frame, or quit if nothing to close
                    if !app.close_current() {
                        app.quit();
                    }
                }
                KeyCode::Esc => {
                    // Close current frame, or quit if nothing to close
                    if !app.close_current() {
                        app.quit();
                    }
                }
                KeyCode::Tab => app.toggle_panel(),
                KeyCode::Char('j') | KeyCode::Down => app.next_item(),
                KeyCode::Char('k') | KeyCode::Up => app.previous_item(),
                KeyCode::Enter => {
                    #[cfg(feature = "imap")]
                    handle_enter(app);
                }
                KeyCode::Char('r') => {
                    #[cfg(feature = "imap")]
                    refresh_current(app);
                }
                KeyCode::Char('c') => {
                    app.start_compose();
                }
                #[cfg(feature = "imap")]
                KeyCode::Char('R') => {
                    handle_reply(app);
                }
                #[cfg(feature = "imap")]
                KeyCode::Char('f') => {
                    handle_forward(app);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_compose_dialog(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => app.dialog_next(),
        KeyCode::Char('k') | KeyCode::Up => app.dialog_previous(),
        KeyCode::Enter => match app.get_selected_compose_action() {
            ComposeAction::SaveToDrafts => {
                #[cfg(feature = "imap")]
                {
                    let content = app.get_compose_content();
                    app.set_status("Saving to Drafts...");
                    match imap::save_to_drafts(&app.imap_config, &content) {
                        Ok(_) => {
                            app.set_status("Saved to Drafts");
                            app.cancel_compose();
                        }
                        Err(e) => app.set_status(format!("Error: {}", e)),
                    }
                }
                #[cfg(not(feature = "imap"))]
                {
                    app.set_status("IMAP feature not enabled");
                    app.cancel_compose();
                }
            }
            ComposeAction::Abandon => {
                app.cancel_compose();
                app.set_status("Composition cancelled");
            }
        },
        KeyCode::Esc | KeyCode::Char('q') => {
            // Go back to editing
            app.show_compose_dialog = false;
        }
        _ => {}
    }
}

#[cfg(feature = "imap")]
fn handle_enter(app: &mut App) {
    match app.active_panel {
        Panel::Mailboxes => {
            app.select_mailbox();
            if let Some(ref mailbox) = app.selected_mailbox {
                match imap::fetch_envelopes(&app.imap_config, mailbox) {
                    Ok(envelopes) => app.set_envelopes(envelopes),
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        Panel::Envelopes => {
            // Fetch and show the selected message
            if let (Some(envelope), Some(mailbox)) = (
                app.get_selected_envelope().cloned(),
                app.selected_mailbox.clone(),
            ) {
                app.set_status(format!("Loading message {}...", envelope.uid));
                match imap::fetch_message(&app.imap_config, &mailbox, envelope.uid) {
                    Ok(content) => app.show_message(content),
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        Panel::Message => {
            // Close message view
            app.close_bottom_panel();
        }
        Panel::Compose => {
            // Handled separately
        }
    }
}

#[cfg(feature = "imap")]
fn handle_reply(app: &mut App) {
    if let (Some(envelope), Some(mailbox)) = (
        app.get_selected_envelope().cloned(),
        app.selected_mailbox.clone(),
    ) {
        app.set_status(format!("Loading message {}...", envelope.uid));
        match imap::fetch_raw_message(&app.imap_config, &mailbox, envelope.uid) {
            Ok(raw) => app.start_reply(&raw),
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
    }
}

#[cfg(feature = "imap")]
fn handle_forward(app: &mut App) {
    if let (Some(envelope), Some(mailbox)) = (
        app.get_selected_envelope().cloned(),
        app.selected_mailbox.clone(),
    ) {
        app.set_status(format!("Loading message {}...", envelope.uid));
        match imap::fetch_raw_message(&app.imap_config, &mailbox, envelope.uid) {
            Ok(raw) => app.start_forward(&raw),
            Err(e) => app.set_status(format!("Error: {}", e)),
        }
    }
}

#[cfg(feature = "imap")]
fn refresh_current(app: &mut App) {
    app.set_status("Refreshing...");
    match imap::fetch_mailboxes(&app.imap_config) {
        Ok(mailboxes) => {
            app.set_mailboxes(mailboxes);
            if let Some(ref mailbox) = app.selected_mailbox.clone() {
                match imap::fetch_envelopes(&app.imap_config, mailbox) {
                    Ok(envelopes) => app.set_envelopes(envelopes),
                    Err(e) => app.set_status(format!("Error: {}", e)),
                }
            }
        }
        Err(e) => app.set_status(format!("Error: {}", e)),
    }
}

fn get_config_paths() -> Vec<PathBuf> {
    if let Ok(paths) = std::env::var("HIMALAYA_CONFIG") {
        paths
            .split(':')
            .filter_map(|p| {
                let expanded = shellexpand::full(p).ok()?;
                Some(PathBuf::from(expanded.as_ref()))
            })
            .collect()
    } else {
        Vec::new()
    }
}
