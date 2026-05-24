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

use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use crate::app::{panel::Panel, state::App};

use super::layout::get_border_style;

pub fn render_message(frame: &mut Frame, app: &App, area: Rect) {
    let content = app
        .message_content
        .as_deref()
        .unwrap_or("No message loaded");

    let lines: Vec<Line> = content.lines().map(Line::from).collect();
    let total_lines = lines.len() as u16;

    let block = Block::default()
        .title(" Message ")
        .borders(Borders::ALL)
        .border_style(get_border_style(app, Panel::Message));

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(inner_height);
    let scroll = app.message_scroll.min(max_scroll);

    // `Wrap { trim: false }` preserves indentation when a line spills
    // over the right edge; long header lines and quoted blocks stay
    // legible instead of falling off the panel.
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false })
        .style(app.theme.message_body);

    frame.render_widget(paragraph, area);

    if total_lines > inner_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(scroll as usize);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}
