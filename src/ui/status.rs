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

//! Bottom-row status strip: shows pending status (loading, errors,
//! confirmations) or, when idle, the current mailbox / message count /
//! contextual key hint.

use ratatui::{Frame, layout::Rect, widgets::Paragraph};

use crate::app::{panel::BottomPanel, state::App};

pub fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        let mailbox = app.selected_mailbox_name().unwrap_or("None");
        let mode_hint = match app.bottom_panel {
            BottomPanel::None => "Enter: select",
            BottomPanel::Message => "Esc: close",
            BottomPanel::MessagePreview => "Esc: back to compose",
            BottomPanel::Compose => "Esc: actions",
        };
        format!(
            " {} | {} msgs | Tab: panel | {}",
            mailbox,
            app.envelopes.len(),
            mode_hint
        )
    };

    let status_bar = Paragraph::new(status).style(app.theme.status_bar);
    frame.render_widget(status_bar, area);
}
