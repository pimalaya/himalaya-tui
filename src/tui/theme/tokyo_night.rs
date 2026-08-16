//! Tokyo Night (Night), a 24-bit RGB palette.
//! bg=#1a1b26, line-bg=#292e42, fg=#c0caf5, comment=#565f89,
//! cyan=#7dcfff, blue=#7aa2f7, purple=#bb9af7, green=#9ece6a,
//! red=#f7768e, orange=#e0af68

use ratatui::style::{Color, Modifier, Style};

use crate::tui::theme::Theme;

const BG: Color = Color::Rgb(0x1a, 0x1b, 0x26);
const LINE_BG: Color = Color::Rgb(0x29, 0x2e, 0x42);
const FG: Color = Color::Rgb(0xc0, 0xca, 0xf5);
const COMMENT: Color = Color::Rgb(0x56, 0x5f, 0x89);
const CYAN: Color = Color::Rgb(0x7d, 0xcf, 0xff);
const BLUE: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const ORANGE: Color = Color::Rgb(0xe0, 0xaf, 0x68);

pub const THEME: Theme = Theme {
    header: Style::new().bg(BLUE).fg(BG).add_modifier(Modifier::BOLD),
    status_bar: Style::new().bg(LINE_BG).fg(FG),
    border_active: Style::new().fg(CYAN),
    border_inactive: Style::new().fg(COMMENT),
    dialog_border: Style::new().fg(ORANGE),
    cursor: Style::new().bg(BLUE).fg(BG).add_modifier(Modifier::BOLD),
    mailbox_current: Style::new().fg(ORANGE).add_modifier(Modifier::BOLD),
    envelope_header: Style::new().fg(ORANGE).add_modifier(Modifier::BOLD),
    envelope_seen: Style::new().fg(COMMENT),
    envelope_unread: Style::new().fg(FG).add_modifier(Modifier::BOLD),
    message_body: Style::new().fg(FG),
    compose_text: Style::new().fg(FG),
    compose_cursor: Style::new().bg(FG).fg(BG),
    compose_selection: Style::new().bg(LINE_BG).fg(FG),
};
