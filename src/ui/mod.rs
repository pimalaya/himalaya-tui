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

//! Ratatui render pipeline: header + three-pane layout (mailboxes,
//! envelopes, message or composer) + status bar, plus the modal
//! dialog overlay used by envelope and compose actions.

mod compose;
mod envelopes;
mod mailboxes;
mod message;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
};

use crate::app::{App, BottomPanelMode, ComposeAction, Dialog, EnvelopeAction, FlagAction, Panel};

pub use compose::render_compose;
pub use envelopes::render_envelopes;
pub use mailboxes::render_mailboxes;
pub use message::render_message;

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

    // Render dialog overlay if needed
    match app.dialog {
        Some(Dialog::Envelope) => render_dialog(
            frame,
            app.dialog_index,
            " Actions ",
            &EnvelopeAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::Compose) => render_dialog(
            frame,
            app.dialog_index,
            " Compose ",
            &ComposeAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::CopyTo) => {
            let labels: Vec<&str> = app.mailboxes.iter().map(|m| m.name.as_str()).collect();
            render_dialog(frame, app.dialog_index, " Copy to ", &labels);
        }
        Some(Dialog::MoveTo) => {
            let labels: Vec<&str> = app.mailboxes.iter().map(|m| m.name.as_str()).collect();
            render_dialog(frame, app.dialog_index, " Move to ", &labels);
        }
        Some(Dialog::FlagAdd) => render_dialog(
            frame,
            app.dialog_index,
            " Add Flag ",
            &FlagAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::FlagRemove) => render_dialog(
            frame,
            app.dialog_index,
            " Remove Flag ",
            &FlagAction::ALL.map(|a| a.label()),
        ),
        None => {}
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" Himalaya TUI — {} ", app.account_name);
    let header = Paragraph::new(title).style(
        Style::default()
            .bg(Color::Blue)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, area);
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
    match app.bottom_panel_mode {
        BottomPanelMode::None => {
            render_envelopes(frame, app, area);
        }
        BottomPanelMode::Message | BottomPanelMode::Compose => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_envelopes(frame, app, chunks[0]);

            match app.bottom_panel_mode {
                BottomPanelMode::Message => render_message(frame, app, chunks[1]),
                BottomPanelMode::Compose => render_compose(frame, app, chunks[1]),
                _ => {}
            }
        }
    }
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status = if let Some(ref msg) = app.status_message {
        msg.clone()
    } else {
        let mailbox = app.selected_mailbox_name().unwrap_or("None");
        let mode_hint = match app.bottom_panel_mode {
            BottomPanelMode::None => "Enter: select",
            BottomPanelMode::Message => "Esc: close",
            BottomPanelMode::Compose => "Esc: actions",
        };
        format!(
            " {} | {} msgs | Tab: panel | {}",
            mailbox,
            app.envelopes.len(),
            mode_hint
        )
    };

    let status_bar =
        Paragraph::new(status).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}

fn render_dialog(frame: &mut Frame, selected_index: usize, title: &str, labels: &[&str]) {
    let height = (labels.len() as u16 + 2).min(20);
    let area = centered_rect_fixed_height(40, height, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let style = if i == selected_index {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
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

pub fn get_border_style(app: &App, panel: Panel) -> Style {
    if app.active_panel == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    }
}
