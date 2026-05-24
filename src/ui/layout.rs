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

//! Top-level frame composition: header + three-pane main area +
//! status bar, with the modal dialog overlay drawn last. Per-pane
//! renderers live in the sibling submodules.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
};

use crate::{
    app::{
        panel::{BottomPanel, Panel},
        state::App,
    },
    ui::{
        compose::render_compose, dialog::render_dialog_overlay, envelopes::render_envelopes,
        header::render_header, mailboxes::render_mailboxes, message::render_message,
        status::render_status_bar,
    },
};

pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_main(frame, app, chunks[1]);
    render_status_bar(frame, app, chunks[2]);
    render_dialog_overlay(frame, app);
}

pub fn get_border_style(app: &App, panel: Panel) -> Style {
    if app.active_panel == panel {
        app.theme.border_active
    } else {
        app.theme.border_inactive
    }
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_mailboxes(frame, app, chunks[0]);
    render_right_panel(frame, app, chunks[1]);
}

fn render_right_panel(frame: &mut Frame, app: &mut App, area: Rect) {
    match app.bottom_panel {
        BottomPanel::None => {
            render_envelopes(frame, app, area);
        }
        BottomPanel::Message | BottomPanel::MessagePreview | BottomPanel::Compose => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_envelopes(frame, app, chunks[0]);

            match app.bottom_panel {
                BottomPanel::Message | BottomPanel::MessagePreview => {
                    render_message(frame, app, chunks[1])
                }
                BottomPanel::Compose => render_compose(frame, app, chunks[1]),
                BottomPanel::None => {}
            }
        }
    }
}
