//! View layer of the Elm Architecture: every render path is rooted at
//! [`render`]. Reads [`Model`] (and adjusts the `*_offset` scroll
//! fields during layout); never produces a [`crate::tui::model::Message`]
//! or touches [`crate::tui::update`].

use edtui::{EditorTheme, EditorView};
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

use crate::{
    email::{
        envelope::Envelope,
        flag::{Flag, IanaFlag},
    },
    tui::{
        command,
        model::{
            BottomPanel, Dialog, FlagAction, Keybinds, MAILBOX_DIALOG_VISIBLE, Model, Panel,
            format_message_count,
        },
        palette,
        theme::Theme,
    },
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
    palette::view::render(frame, model);
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
            BottomPanel::None => "Enter: select",
            BottomPanel::Message => "Esc: close",
            BottomPanel::MessagePreview => "Esc: back to compose",
            BottomPanel::Compose => "Esc: actions",
        };
        let message_count = model.envelope_total.unwrap_or(model.envelopes.len() as u64);
        format!(" {mailbox} | {message_count} msgs | Tab: panel | {mode_hint}")
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
            let is_selected = model.is_selected(envelope);
            let style = if is_selected {
                model.theme.mailbox_current
            } else if envelope.flags.contains(&Flag::from_iana(IanaFlag::Seen)) {
                model.theme.envelope_seen
            } else {
                model.theme.envelope_unread
            };
            let marker = if is_selected { "▌ " } else { "  " };

            let cells = vec![
                Cell::from(format!("{marker}{}", format_flags(envelope))),
                Cell::from(envelope.subject.clone()),
                Cell::from(truncate(&format_from(envelope), 20)),
                Cell::from(truncate(&format_date(envelope), 6)),
            ];

            Row::new(cells).style(style)
        })
        .collect();

    let selection_badge = if model.has_selection() {
        format!(" — {} selected", model.selection_count())
    } else {
        String::new()
    };
    let block = Block::default()
        .title(format!(
            " Envelopes{}{} ",
            model
                .selected_mailbox_name()
                .map(|m| {
                    let page = model.envelope_page + 1;
                    match model.total_pages() {
                        Some(count) if count > 1 => format!(" - {m} ({page}/{count})"),
                        Some(_) => format!(" - {m}"),
                        None => format!(" - {m} ({page})"),
                    }
                })
                .unwrap_or_default(),
            selection_badge
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
    let editor_hint = match model.keybinds {
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
    match model.dialog {
        Some(dialog @ (Dialog::Envelope | Dialog::Compose)) => {
            render_command_dialog(frame, model, dialog)
        }
        Some(dialog @ (Dialog::CopyTo | Dialog::MoveTo)) => {
            let verb = if dialog == Dialog::CopyTo {
                "Copy"
            } else {
                "Move"
            };
            render_mailbox_dialog(frame, model, &transfer_dialog_title(model, verb));
        }
        Some(dialog @ (Dialog::FlagAdd | Dialog::FlagRemove)) => {
            let base = if dialog == Dialog::FlagAdd {
                "Add Flag"
            } else {
                "Remove Flag"
            };
            render_dialog(
                frame,
                &model.theme,
                model.dialog_index,
                &flag_dialog_title(model, base),
                &FlagAction::ALL.map(|a| (a.label(), String::new())),
            );
        }
        Some(Dialog::SwitchAccount) => render_switch_account_dialog(frame, model),
        None => {}
    }
}

/// Rows mirror `switch_account_candidates` 1:1 (both read
/// `account_names`); the hint column marks the active account.
fn render_switch_account_dialog(frame: &mut Frame, model: &Model) {
    let rows: Vec<(&str, String)> = model
        .account_names
        .iter()
        .map(|name| {
            let hint = if *name == model.account_name {
                String::from("current")
            } else {
                String::new()
            };
            (name.as_str(), hint)
        })
        .collect();
    render_dialog(
        frame,
        &model.theme,
        model.dialog_index,
        " Switch Account ",
        &rows,
    );
}

fn transfer_dialog_title(model: &Model, verb: &str) -> String {
    match model.bulk_target_count() {
        0 | 1 => format!(" {verb} to "),
        count => format!(" {verb} {} to ", format_message_count(count)),
    }
}

fn flag_dialog_title(model: &Model, base: &str) -> String {
    match model.bulk_target_count() {
        0 | 1 => format!(" {base} "),
        count => format!(" {base} ({}) ", format_message_count(count)),
    }
}

fn render_command_dialog(frame: &mut Frame, model: &Model, dialog: Dialog) {
    let Some(context) = dialog.command_context() else {
        return;
    };
    let title = if dialog == Dialog::Envelope {
        " Actions "
    } else {
        " Compose "
    };
    let rows: Vec<(&str, String)> = command::for_context(context)
        .map(|c| (c.name, c.hint(model.keybinds).unwrap_or_default()))
        .collect();
    render_dialog(frame, &model.theme, model.dialog_index, title, &rows);
}

/// Two centered stacked frames: top has the title + a `> ` prompt
/// and the filter input; bottom is an untitled, fixed-height results
/// frame so the dialog size does not jump as the filter narrows.
fn render_mailbox_dialog(frame: &mut Frame, model: &Model, title: &str) {
    let list_inner = render_filter_overlay(
        frame,
        &model.theme,
        title,
        &model.mailbox_filter,
        MAILBOX_DIALOG_VISIBLE,
    );

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

/// Centered filter-picker chrome shared by the mailbox picker and the
/// command palette: a titled input frame with a `> ` prompt and live
/// cursor, stacked on a fixed-height untitled result frame (fixed so
/// the overlay does not resize as the filter narrows). Returns the
/// result frame's inner area for the caller to fill.
pub(crate) fn render_filter_overlay(
    frame: &mut Frame,
    theme: &Theme,
    title: &str,
    filter: &tui_input::Input,
    visible_rows: usize,
) -> Rect {
    const INPUT_BOX_HEIGHT: u16 = 3;
    const PROMPT: &str = "> ";
    const WIDTH_PERCENT: u16 = 40;

    let list_box_height = visible_rows as u16 + 2;
    let total_height = INPUT_BOX_HEIGHT + list_box_height;
    let area = centered_rect_fixed_height(WIDTH_PERCENT, total_height, frame.area());

    frame.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(INPUT_BOX_HEIGHT),
            Constraint::Length(list_box_height),
        ])
        .split(area);

    let input_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.dialog_border);
    let input_inner = input_block.inner(chunks[0]);
    frame.render_widget(input_block, chunks[0]);

    frame.render_widget(
        Paragraph::new(format!("{PROMPT}{}", filter.value())),
        input_inner,
    );

    let cursor_col = input_inner.x
        + (PROMPT.len() as u16 + filter.visual_cursor() as u16)
            .min(input_inner.width.saturating_sub(1));
    frame.set_cursor_position((cursor_col, input_inner.y));

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme.dialog_border);
    let list_inner = list_block.inner(chunks[1]);
    frame.render_widget(list_block, chunks[1]);

    list_inner
}

fn render_dialog(
    frame: &mut Frame,
    theme: &Theme,
    selected_index: usize,
    title: &str,
    rows: &[(&str, String)],
) {
    let height = (rows.len() as u16 + 2).min(20);
    let area = centered_rect_fixed_height(40, height, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(theme.dialog_border);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, (label, hint))| {
            let style = if i == selected_index {
                theme.cursor
            } else {
                theme.message_body
            };

            let text = aligned_row(label, hint, i == selected_index, inner.width);

            ListItem::new(Line::from(Span::styled(text, style)))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

/// One picker/menu row: selection prefix + `label`, with `hint`
/// right-aligned to `width`.
pub(crate) fn aligned_row(label: &str, hint: &str, is_selected: bool, width: u16) -> String {
    let prefix = if is_selected { "> " } else { "  " };
    let label_width = (width as usize).saturating_sub(prefix.len() + hint.len());
    format!("{prefix}{label:<label_width$}{hint}")
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
