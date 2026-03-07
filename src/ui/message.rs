use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

use crate::app::{App, Panel};

use super::get_border_style;

pub fn render_message(frame: &mut Frame, app: &App, area: Rect) {
    let content = app.message_content.as_deref().unwrap_or("No message loaded");

    let lines: Vec<Line> = content.lines().map(Line::from).collect();
    let total_lines = lines.len() as u16;

    let block = Block::default()
        .title(" Message ")
        .borders(Borders::ALL)
        .border_style(get_border_style(app, Panel::Message));

    let inner_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(inner_height);
    let scroll = app.message_scroll.min(max_scroll);

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((scroll, 0))
        .style(Style::default().fg(Color::White));

    frame.render_widget(paragraph, area);

    // Render scrollbar if content exceeds visible area
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
