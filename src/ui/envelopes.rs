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

use io_email::{
    envelope::Envelope,
    flag::{Flag, IanaFlag},
};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};

use super::layout::get_border_style;
use crate::app::{panel::Panel, state::App};

pub fn render_envelopes(frame: &mut Frame, app: &mut App, area: Rect) {
    let header_cells = ["Flags", "Subject", "From", "Date"].map(Cell::from);
    let header = Row::new(header_cells)
        .style(app.theme.envelope_header)
        .height(1);

    let rows: Vec<Row> = app
        .envelopes
        .iter()
        .map(|envelope| {
            let style = if envelope.flags.contains(&Flag::from_iana(IanaFlag::Seen)) {
                app.theme.envelope_seen
            } else {
                app.theme.envelope_unread
            };

            let cells = vec![
                Cell::from(format_flags(envelope)),
                Cell::from(envelope.subject.clone()),
                Cell::from(truncate(&format_from(envelope), 20)),
                Cell::from(truncate(&format_date(envelope), 6)),
            ];

            Row::new(cells).style(style)
        })
        .collect();

    let block = Block::default()
        .title(format!(
            " Envelopes{} ",
            app.selected_mailbox_name()
                .map(|m| {
                    let total_pages = app.total_pages();
                    if total_pages > 1 {
                        format!(" - {} ({}/{})", m, app.envelope_page + 1, total_pages)
                    } else {
                        format!(" - {}", m)
                    }
                })
                .unwrap_or_default()
        ))
        .borders(Borders::ALL)
        .border_style(get_border_style(app, Panel::Envelopes));

    let widths = [
        Constraint::Length(6),
        Constraint::Min(10),
        Constraint::Length(20),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(app.theme.cursor);

    // Page-style scrolling: when the cursor leaves the visible
    // window, snap the offset so the new selection sits at the page
    // edge (top when going down, bottom when going up).
    //
    // Inner height accounts for top/bottom borders (-2) and the
    // header row (-1).
    let inner_height = area.height.saturating_sub(3) as usize;
    if inner_height > 0 {
        if app.envelope_index >= app.envelope_offset + inner_height {
            app.envelope_offset = app.envelope_index;
        } else if app.envelope_index < app.envelope_offset {
            app.envelope_offset = app
                .envelope_index
                .saturating_sub(inner_height.saturating_sub(1));
        }
    }

    let mut state = TableState::default().with_offset(app.envelope_offset);
    if app.active_panel == Panel::Envelopes {
        state.select(Some(app.envelope_index));
    }
    frame.render_stateful_widget(table, area, &mut state);
}

fn format_flags(envelope: &Envelope) -> String {
    let mut s = String::new();
    s.push(
        if envelope.flags.contains(&Flag::from_iana(IanaFlag::Seen)) {
            ' '
        } else {
            '*'
        },
    );
    s.push(
        if envelope
            .flags
            .contains(&Flag::from_iana(IanaFlag::Answered))
        {
            '↩'
        } else {
            ' '
        },
    );
    s.push(
        if envelope.flags.contains(&Flag::from_iana(IanaFlag::Flagged)) {
            '!'
        } else {
            ' '
        },
    );
    s.push(
        if envelope.flags.contains(&Flag::from_iana(IanaFlag::Draft)) {
            'D'
        } else {
            ' '
        },
    );
    s
}

fn format_from(envelope: &Envelope) -> String {
    envelope
        .from
        .first()
        .map(|addr| addr.name.clone().unwrap_or_else(|| addr.email.clone()))
        .unwrap_or_default()
}

fn format_date(envelope: &Envelope) -> String {
    envelope
        .date
        .map(|d| d.format("%d %b").to_string())
        .unwrap_or_default()
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}...", s.chars().take(max_len - 3).collect::<String>())
    }
}
