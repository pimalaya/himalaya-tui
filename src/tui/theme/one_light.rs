//! # One Light theme
//!
//! Light theme built from the 24-bit RGB Atom One Light palette.

use ratatui::style::{Color, Modifier, Style};

use crate::tui::theme::Theme;

const BG: Color = Color::Rgb(0xfa, 0xfa, 0xfa);
const LINE_BG: Color = Color::Rgb(0xe5, 0xe5, 0xe6);
const MONO_1: Color = Color::Rgb(0x38, 0x3a, 0x42);
const MONO_3: Color = Color::Rgb(0xa0, 0xa1, 0xa7);
const CYAN: Color = Color::Rgb(0x01, 0x84, 0xbc);
const BLUE: Color = Color::Rgb(0x40, 0x78, 0xf2);
const ORANGE: Color = Color::Rgb(0xc1, 0x84, 0x01);

/// Theme built from the Atom One Light palette.
pub const THEME: Theme = Theme {
    header: Style::new().bg(BLUE).fg(BG).add_modifier(Modifier::BOLD),
    status_bar: Style::new().bg(LINE_BG).fg(MONO_1),
    border_active: Style::new().fg(CYAN),
    border_inactive: Style::new().fg(MONO_3),
    dialog_border: Style::new().fg(ORANGE),
    cursor: Style::new().bg(BLUE).fg(BG).add_modifier(Modifier::BOLD),
    mailbox_current: Style::new().fg(ORANGE).add_modifier(Modifier::BOLD),
    envelope_header: Style::new().fg(ORANGE).add_modifier(Modifier::BOLD),
    envelope_seen: Style::new().fg(MONO_3),
    envelope_unread: Style::new().fg(MONO_1).add_modifier(Modifier::BOLD),
    message_body: Style::new().fg(MONO_1),
    compose_text: Style::new().fg(MONO_1),
    compose_cursor: Style::new().bg(MONO_1).fg(BG),
    compose_selection: Style::new().bg(LINE_BG).fg(MONO_1),
};
