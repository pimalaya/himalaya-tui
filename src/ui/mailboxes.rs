// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use super::get_border_style;
use crate::app::{App, Panel};

pub fn render_mailboxes(frame: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .mailboxes
        .iter()
        .map(|mailbox| {
            let style = if Some(&mailbox.id) == app.selected_mailbox.as_ref() {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(mailbox.name.clone(), style)))
        })
        .collect();

    let block = Block::default()
        .title(" Mailboxes ")
        .borders(Borders::ALL)
        .border_style(get_border_style(app, Panel::Mailboxes));

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(Color::Cyan)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );

    // Page-style scrolling: when the cursor leaves the visible
    // window, snap the offset so the new selection sits at the page
    // edge (top when going down, bottom when going up).
    //
    // Inner height accounts for top/bottom borders (-2). No header.
    let inner_height = area.height.saturating_sub(2) as usize;
    if inner_height > 0 {
        if app.mailbox_index >= app.mailbox_offset + inner_height {
            app.mailbox_offset = app.mailbox_index;
        } else if app.mailbox_index < app.mailbox_offset {
            app.mailbox_offset = app
                .mailbox_index
                .saturating_sub(inner_height.saturating_sub(1));
        }
    }

    let mut state = ListState::default().with_offset(app.mailbox_offset);
    if app.active_panel == Panel::Mailboxes {
        state.select(Some(app.mailbox_index));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
