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

//! Event loop driving the Model-Update-View cycle. Owns terminal
//! setup, raw-mode lifecycle and the system-editor handoff (which
//! needs `&mut Terminal`, so cannot live inside [`crate::tui::update`]).

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

const POLL_TIMEOUT: Duration = Duration::from_millis(250);

pub fn run(mut model: Model) -> Result<()> {
    // Restore the terminal on panic so the user is not stuck with raw
    // mode and the alternate screen on a crash.
    let panic_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        restore_terminal().unwrap();
        panic_hook(info);
    }));

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    while model.running {
        // The composer queues a system-editor request via OpenSystemEditor;
        // edtui flags the state, and we flush it here before the next draw
        // because the open call needs &mut Terminal.
        if model.active_panel == Panel::Compose && system_editor::is_pending(&model.editor_state) {
            system_editor::open(&mut model.editor_state, &mut terminal)?;
        }

        terminal.draw(|f| view::render(&mut model, f))?;

        if !event::poll(POLL_TIMEOUT)? {
            // Idle tick: keep network backends warm so the server
            // does not drop the connection mid-session.
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

fn restore_terminal() -> Result<()> {
    stdout().execute(LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
