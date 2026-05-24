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

//! Top-row header strip: shows the active account name.

use ratatui::{Frame, layout::Rect, widgets::Paragraph};

use crate::app::state::App;

pub fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" Himalaya TUI — {} ", app.account_name);
    let header = Paragraph::new(title).style(app.theme.header);
    frame.render_widget(header, area);
}
