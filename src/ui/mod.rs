mod compose;
mod envelopes;
mod mailboxes;
mod message;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, BottomPanelMode, ComposeAction, Panel};

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
    if app.show_compose_dialog {
        render_compose_dialog(frame, app);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(" Himalaya TUI - {} ", app.account_name);
    let header = Paragraph::new(title)
        .style(Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD));
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
        let mailbox = app.selected_mailbox.as_deref().unwrap_or("None");
        let mode_hint = match app.bottom_panel_mode {
            BottomPanelMode::None => "c: compose | R: reply | f: forward | Enter: read",
            BottomPanelMode::Message => "q: close | c: compose | R: reply | f: forward",
            BottomPanelMode::Compose => "C-c C-c: finish",
        };
        format!(
            " {} | {} msgs | q: close | Tab: panel | j/k: nav | {}",
            mailbox,
            app.envelopes.len(),
            mode_hint
        )
    };

    let status_bar =
        Paragraph::new(status).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(status_bar, area);
}

fn render_compose_dialog(frame: &mut Frame, app: &App) {
    let area = centered_rect(40, 25, frame.area());

    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Finish Composition ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Build selectable list items
    let items: Vec<ListItem> = ComposeAction::ALL
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let style = if i == app.compose_dialog_index {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            let prefix = if i == app.compose_dialog_index {
                "> "
            } else {
                "  "
            };

            ListItem::new(Line::from(Span::styled(
                format!("{}{}", prefix, action.label()),
                style,
            )))
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn get_border_style(app: &App, panel: Panel) -> Style {
    if app.active_panel == panel {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Gray)
    }
}
