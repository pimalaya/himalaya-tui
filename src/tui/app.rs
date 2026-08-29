//! # App
//!
//! Event loop driving the Model-Update-View cycle. It owns terminal
//! setup, the raw-mode lifecycle and the system-editor handoff.

use std::{io::stdout, panic, time::Duration};

use anyhow::Result;
use edtui::system_editor;
use ratatui::{
    Terminal,
    crossterm::{
        ExecutableCommand,
        event::{self, Event, KeyEventKind},
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    prelude::CrosstermBackend,
};

use crate::tui::{
    model::{Message, Model, PING_INTERVAL, Panel},
    update, view,
};

/// How long an iteration waits for an event before ticking idle.
const POLL_TIMEOUT: Duration = Duration::from_millis(250);

/// Runs the loop until the model stops running, then restores the
/// terminal.
pub fn run(mut model: Model) -> Result<()> {
    // A panic escaping the loop would otherwise leave the terminal in
    // raw mode, on the alternate screen.
    let panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal().unwrap();
        panic_hook(info);
    }));

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    while model.running {
        // Flushed here rather than in update: opening the editor needs
        // &mut Terminal, which only this loop holds.
        if model.active_panel == Panel::Compose && system_editor::is_pending(&model.editor_state) {
            system_editor::open(&mut model.editor_state, &mut terminal)?;
        }

        terminal.draw(|f| view::render(&mut model, f))?;

        // A page is one screenful, so the render that measured the screen
        // decides its size: the first render, and every one after a
        // resize, re-pages the list around the cursor.
        let repage = update::adopt_envelope_capacity(&mut model);
        update::apply_all(&mut model, repage);

        if !event::poll(POLL_TIMEOUT)? {
            if model.last_activity.elapsed() >= PING_INTERVAL {
                update::apply_all(&mut model, Some(Message::Ping));
            }
            continue;
        }

        if let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            update::apply_all(&mut model, Some(Message::Key(key)));
        }
    }

    restore_terminal()
}

/// Leaves the alternate screen and disables raw mode.
fn restore_terminal() -> Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
