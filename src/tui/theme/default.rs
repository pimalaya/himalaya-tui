//! # Default theme
//!
//! Built-in theme using named ANSI colors, letting the user's terminal
//! palette decide the actual shades so the TUI blends with whatever
//! color scheme they already use.

use ratatui::style::{Color, Modifier, Style};

use crate::tui::theme::Theme;

/// Theme built from the terminal ANSI colors.
pub const THEME: Theme = Theme {
    header: Style::new()
        .bg(Color::Blue)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD),
    status_bar: Style::new().bg(Color::DarkGray).fg(Color::White),
    border_active: Style::new().fg(Color::Cyan),
    border_inactive: Style::new().fg(Color::Gray),
    dialog_border: Style::new().fg(Color::Yellow),
    cursor: Style::new()
        .bg(Color::Cyan)
        .fg(Color::White)
        .add_modifier(Modifier::BOLD),
    mailbox_current: Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    envelope_header: Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    envelope_seen: Style::new().fg(Color::Gray),
    envelope_unread: Style::new().add_modifier(Modifier::BOLD),
    message_body: Style::new().fg(Color::White),
    compose_text: Style::new().fg(Color::White),
    compose_cursor: Style::new().bg(Color::White).fg(Color::Black),
    compose_selection: Style::new().bg(Color::Cyan).fg(Color::Black),
};
