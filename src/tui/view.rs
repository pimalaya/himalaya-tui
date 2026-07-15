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

//! View layer of the Elm Architecture: every render path is rooted at
//! [`render`]. Reads [`Model`] (and adjusts the `*_offset` scroll
//! fields during layout); never produces a [`crate::tui::model::Message`]
//! or touches [`crate::tui::update`].

use edtui::{EditorTheme, EditorView};
use io_email::{
    envelope::types::Envelope,
    flag::types::{Flag, IanaFlag},
};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState, Widget, Wrap,
    },
};

use crate::tui::{
    model::{
        BottomPanel, ComposeAction, Dialog, EnvelopeAction, FlagAction, Keybinds,
        MAILBOX_DIALOG_VISIBLE, Model, Panel,
    },
    theme::Theme,
};

/// View entry point. Lays out the header, the three-pane main area
/// and the status bar, then overlays any open modal dialog.
pub fn render(model: &mut Model, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_header(frame, model, chunks[0]);
    render_main(frame, model, chunks[1]);
    render_status_bar(frame, model, chunks[2]);
    render_dialog_overlay(frame, model);
}

fn render_main(frame: &mut Frame, model: &mut Model, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(area);

    render_mailboxes(frame, model, chunks[0]);
    render_right_panel(frame, model, chunks[1]);
}

fn render_right_panel(frame: &mut Frame, model: &mut Model, area: Rect) {
    match model.bottom_panel {
        BottomPanel::None => {
            render_envelopes(frame, model, area);
        }
        BottomPanel::Message | BottomPanel::MessagePreview | BottomPanel::Compose => {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(area);

            render_envelopes(frame, model, chunks[0]);

            match model.bottom_panel {
                BottomPanel::Message | BottomPanel::MessagePreview => {
                    render_message(frame, model, chunks[1])
                }
                BottomPanel::Compose => render_compose(frame, model, chunks[1]),
                BottomPanel::None => {}
            }
        }
    }
}

fn get_border_style(model: &Model, panel: Panel) -> Style {
    if model.active_panel == panel {
        model.theme.border_active
    } else {
        model.theme.border_inactive
    }
}

fn render_header(frame: &mut Frame, model: &Model, area: Rect) {
    let title = format!(" Himalaya TUI — {} ", model.account_name);
    let header = Paragraph::new(title).style(model.theme.header);
    frame.render_widget(header, area);
}

fn render_status_bar(frame: &mut Frame, model: &Model, area: Rect) {
    let status = if let Some(ref msg) = model.status_message {
        msg.clone()
    } else {
        let mailbox = model.selected_mailbox_name().unwrap_or("None");
        let mode_hint = match model.bottom_panel {
            BottomPanel::None => "Enter: select, Shift+l: re(L)oad",
            BottomPanel::Message => "Esc: close, Shift+r: (R)eply, Shift+a: reply (A)ll",
            BottomPanel::MessagePreview => "Esc: back to compose",
            BottomPanel::Compose => "Esc: actions",
        };
        format!(
            " {} | {} msgs | Tab: panel | {}",
            mailbox,
            model.envelopes.len(),
            mode_hint
        )
    };

    let status_bar = Paragraph::new(status).style(model.theme.status_bar);
    frame.render_widget(status_bar, area);
}

fn render_mailboxes(frame: &mut Frame, model: &mut Model, area: Rect) {
    let items: Vec<ListItem> = model
        .mailboxes
        .iter()
        .map(|mailbox| {
            let style = if Some(&mailbox.id) == model.selected_mailbox.as_ref() {
                model.theme.mailbox_current
            } else {
                Style::default()
            };

            ListItem::new(Line::from(Span::styled(mailbox.name.clone(), style)))
        })
        .collect();

    let block = Block::default()
        .title(" Mailboxes ")
        .borders(Borders::ALL)
        .border_style(get_border_style(model, Panel::Mailboxes));

    let list = List::new(items)
        .block(block)
        .highlight_style(model.theme.cursor);

    // Page-style scrolling: when the cursor leaves the visible
    // window, snap the offset so the new selection sits at the page
    // edge. Inner height = total height minus top+bottom borders.
    let inner_height = area.height.saturating_sub(2) as usize;
    if inner_height > 0 {
        if model.mailbox_index >= model.mailbox_offset + inner_height {
            model.mailbox_offset = model.mailbox_index;
        } else if model.mailbox_index < model.mailbox_offset {
            model.mailbox_offset = model
                .mailbox_index
                .saturating_sub(inner_height.saturating_sub(1));
        }
    }

    let mut state = ListState::default().with_offset(model.mailbox_offset);
    if model.active_panel == Panel::Mailboxes {
        state.select(Some(model.mailbox_index));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_envelopes(frame: &mut Frame, model: &mut Model, area: Rect) {
    let header_cells = ["Flags", "Subject", "From", "Date"].map(Cell::from);
    let header = Row::new(header_cells)
        .style(model.theme.envelope_header)
        .height(1);

    let rows: Vec<Row> = model
        .envelopes
        .iter()
        .map(|envelope| {
            let style = if envelope.flags.contains(&Flag::from_iana(IanaFlag::Seen)) {
                model.theme.envelope_seen
            } else {
                model.theme.envelope_unread
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
            model
                .selected_mailbox_name()
                .map(|m| {
                    let total_pages = model.total_pages();
                    if total_pages > 1 {
                        format!(" - {} ({}/{})", m, model.envelope_page + 1, total_pages)
                    } else {
                        format!(" - {}", m)
                    }
                })
                .unwrap_or_default()
        ))
        .borders(Borders::ALL)
        .border_style(get_border_style(model, Panel::Envelopes));

    let widths = [
        Constraint::Length(6),
        Constraint::Min(10),
        Constraint::Length(20),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(model.theme.cursor);

    // Page-style scrolling, as in render_mailboxes; inner height also
    // subtracts the table header row.
    let inner_height = area.height.saturating_sub(3) as usize;
    if inner_height > 0 {
        if model.envelope_index >= model.envelope_offset + inner_height {
            model.envelope_offset = model.envelope_index;
        } else if model.envelope_index < model.envelope_offset {
            model.envelope_offset = model
                .envelope_index
                .saturating_sub(inner_height.saturating_sub(1));
        }
    }

    let mut state = TableState::default().with_offset(model.envelope_offset);
    if model.active_panel == Panel::Envelopes {
        state.select(Some(model.envelope_index));
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
        format!("{}…", s.chars().take(max_len - 1).collect::<String>())
    }
}

fn render_message(frame: &mut Frame, model: &Model, area: Rect) {
    let content = model
        .message_content
        .as_deref()
        .unwrap_or("No message loaded");

    let lines: Vec<Line> = content.lines().map(Line::from).collect();
    let total_lines = lines.len() as u16;

    let block = Block::default()
        .title(" Message ")
        .borders(Borders::ALL)
        .border_style(get_border_style(model, Panel::Message));

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(inner_height);
    let scroll = model.message_scroll.min(max_scroll);

    // `trim: false` keeps indentation on wrapped lines so quoted
    // blocks and long headers stay legible.
    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((scroll, 0))
        .wrap(Wrap { trim: false })
        .style(model.theme.message_body);

    frame.render_widget(paragraph, area);

    if total_lines > inner_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state =
            ScrollbarState::new(max_scroll as usize).position(scroll as usize);

        frame.render_stateful_widget(
            scrollbar,
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut scrollbar_state,
        );
    }
}

fn render_compose(frame: &mut Frame, model: &mut Model, area: Rect) {
    // Emacs binds `Ctrl-e` to "move to end of line", so only Vim
    // exposes it as the system-editor shortcut (alongside `Alt-e`).
    let editor_hint = match model.keybinds.unwrap_or_default() {
        Keybinds::Vim => "Ctrl-e or Alt-e: open in $EDITOR",
        Keybinds::Emacs => "Alt-e: open in $EDITOR",
    };
    let title = format!(" Compose (Esc: actions, {editor_hint}) ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(get_border_style(model, Panel::Compose));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let editor_theme = EditorTheme::default()
        .base(model.theme.compose_text)
        .cursor_style(model.theme.compose_cursor)
        .selection_style(model.theme.compose_selection)
        .hide_status_line();

    let buf = frame.buffer_mut();
    EditorView::new(&mut model.editor_state)
        .theme(editor_theme)
        .render(inner, buf);
}

fn render_dialog_overlay(frame: &mut Frame, model: &Model) {
    let theme = model.theme;
    match model.dialog {
        Some(Dialog::Envelope) => render_dialog(
            frame,
            &theme,
            model.dialog_index,
            " Actions ",
            &EnvelopeAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::Compose) => render_dialog(
            frame,
            &theme,
            model.dialog_index,
            " Compose ",
            &ComposeAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::CopyTo) => render_mailbox_dialog(frame, model, " Copy to "),
        Some(Dialog::MoveTo) => render_mailbox_dialog(frame, model, " Move to "),
        Some(Dialog::FlagAdd) => render_dialog(
            frame,
            &theme,
            model.dialog_index,
            " Add Flag ",
            &FlagAction::ALL.map(|a| a.label()),
        ),
        Some(Dialog::FlagRemove) => render_dialog(
            frame,
            &theme,
            model.dialog_index,
            " Remove Flag ",
            &FlagAction::ALL.map(|a| a.label()),
        ),
        None => {}
    }
}

/// Two centered stacked frames: top has the title + a `> ` prompt
/// and the filter input; bottom is an untitled, fixed-height results
/// frame so the dialog size does not jump as the filter narrows.
fn render_mailbox_dialog(frame: &mut Frame, model: &Model, title: &str) {
    const INPUT_BOX_HEIGHT: u16 = 3;
    const PROMPT: &str = "> ";

    let list_box_height = MAILBOX_DIALOG_VISIBLE as u16 + 2;

    let total_height = INPUT_BOX_HEIGHT + list_box_height;
    let area = centered_rect_fixed_height(40, total_height, frame.area());

    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(INPUT_BOX_HEIGHT),
            Constraint::Length(list_box_height),
        ])
        .split(area);

    let input_area = chunks[0];
    let list_area = chunks[1];

    let input_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(model.theme.dialog_border);
    let input_inner = input_block.inner(input_area);
    frame.render_widget(input_block, input_area);

    let filter_value = model.mailbox_filter.value();
    frame.render_widget(
        Paragraph::new(format!("{PROMPT}{filter_value}")),
        input_inner,
    );

    let cursor_col = input_inner.x
        + (PROMPT.len() as u16 + model.mailbox_filter.visual_cursor() as u16)
            .min(input_inner.width.saturating_sub(1));
    frame.set_cursor_position((cursor_col, input_inner.y));

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(model.theme.dialog_border);
    let list_inner = list_block.inner(list_area);
    frame.render_widget(list_block, list_area);

    let items: Vec<ListItem> = model
        .filtered_mailboxes()
        .iter()
        .take(MAILBOX_DIALOG_VISIBLE)
        .enumerate()
        .map(|(i, m)| {
            ListItem::new(Line::from(if i == model.dialog_index {
                Span::styled(format!("> {}", m.name), model.theme.cursor)
            } else {
                Span::styled(&m.name, model.theme.message_body)
            }))
        })
        .collect();

    frame.render_widget(List::new(items), list_inner);
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
