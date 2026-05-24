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

//! Modal dialog overlay: centred panel with a labelled action list,
//! one row per option, the current selection highlighted with the
//! cursor style.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::{
    app::{
        compose::ComposeAction,
        dialog::{Dialog, FlagAction},
        envelopes::EnvelopeAction,
        state::App,
    },
    theme::Theme,
};

pub fn render_dialog_overlay(frame: &mut Frame, app: &App) {
    let theme = app.theme;
    match app.dialog {
        Some(Dialog::Envelope) => render_dialog(
            frame,
            &theme,
            app.dialog_index,
            " Actions ",
            &EnvelopeAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::Compose) => render_dialog(
            frame,
            &theme,
            app.dialog_index,
            " Compose ",
            &ComposeAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::CopyTo) => {
            let labels: Vec<&str> = app.mailboxes.iter().map(|m| m.name.as_str()).collect();
            render_dialog(frame, &theme, app.dialog_index, " Copy to ", &labels);
        }
        Some(Dialog::MoveTo) => {
            let labels: Vec<&str> = app.mailboxes.iter().map(|m| m.name.as_str()).collect();
            render_dialog(frame, &theme, app.dialog_index, " Move to ", &labels);
        }
        Some(Dialog::FlagAdd) => render_dialog(
            frame,
            &theme,
            app.dialog_index,
            " Add Flag ",
            &FlagAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::FlagRemove) => render_dialog(
            frame,
            &theme,
            app.dialog_index,
            " Remove Flag ",
            &FlagAction::ALL.map(|a| a.label()),
        ),
        None => {}
    }
}

fn render_dialog(
    frame: &mut Frame,
    theme: &Theme,
    selected_index: usize,
    title: &str,
    labels: &[&str],
) {
    let height = (labels.len() as u16 + 2).min(20);
    let area = centered_rect_fixed_height(40, height, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.dialog_border);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let style = if i == selected_index {
                theme.cursor
            } else {
                theme.message_body
            };

            let prefix = if i == selected_index { "> " } else { "  " };

            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, label),
                style,
            )))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn centered_rect_fixed_height(percent_x: u16, height: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Fill(1),
            Constraint::Length(height),
            Constraint::Fill(1),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}
