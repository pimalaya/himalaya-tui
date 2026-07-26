//! Dracula 24-bit RGB palette (<https://draculatheme.com/contribute>).
//! bg=#282a36, current-line=#44475a, fg=#f8f8f2, comment=#6272a4,
//! cyan=#8be9fd, green=#50fa7b, orange=#ffb86c, pink=#ff79c6,
//! purple=#bd93f9, red=#ff5555, yellow=#f1fa8c

use ratatui::style::{Color, Modifier, Style};

use crate::tui::theme::Theme;

const FG: Color = Color::Rgb(0xf8, 0xf8, 0xf2);
const BG: Color = Color::Rgb(0x28, 0x2a, 0x36);
const CURRENT_LINE: Color = Color::Rgb(0x44, 0x47, 0x5a);
const COMMENT: Color = Color::Rgb(0x62, 0x72, 0xa4);
const PINK: Color = Color::Rgb(0xff, 0x79, 0xc6);
const PURPLE: Color = Color::Rgb(0xbd, 0x93, 0xf9);
const YELLOW: Color = Color::Rgb(0xf1, 0xfa, 0x8c);

pub const THEME: Theme = Theme {
    header: Style::new().bg(PURPLE).fg(FG).add_modifier(Modifier::BOLD),
    status_bar: Style::new().bg(CURRENT_LINE).fg(FG),
    border_active: Style::new().fg(PINK),
    border_inactive: Style::new().fg(COMMENT),
    dialog_border: Style::new().fg(YELLOW),
    cursor: Style::new().bg(PURPLE).fg(FG).add_modifier(Modifier::BOLD),
    mailbox_current: Style::new().fg(PINK).add_modifier(Modifier::BOLD),
    envelope_header: Style::new().fg(YELLOW).add_modifier(Modifier::BOLD),
    envelope_seen: Style::new().fg(COMMENT),
    envelope_unread: Style::new().fg(FG).add_modifier(Modifier::BOLD),
    message_body: Style::new().fg(FG),
    compose_text: Style::new().fg(FG),
    compose_cursor: Style::new().bg(FG).fg(BG),
    compose_selection: Style::new().bg(CURRENT_LINE).fg(FG),
};
