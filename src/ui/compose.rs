use edtui::{EditorTheme, EditorView};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Widget},
    Frame,
};

use crate::app::{App, Panel};

use super::get_border_style;

pub fn render_compose(frame: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Compose (C-c C-c: finish) ")
        .borders(Borders::ALL)
        .border_style(get_border_style(app, Panel::Compose));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Create a theme that matches the TUI styling
    let theme = EditorTheme::default()
        .base(Style::default().bg(Color::Reset).fg(Color::White))
        .cursor_style(Style::default().bg(Color::White).fg(Color::Black))
        .selection_style(Style::default().bg(Color::Cyan).fg(Color::Black))
        .hide_status_line();

    let buf = frame.buffer_mut();
    EditorView::new(&mut app.editor_state)
        .theme(theme)
        .render(inner, buf);
}
