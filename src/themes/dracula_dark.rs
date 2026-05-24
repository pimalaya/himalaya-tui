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

//! Dracula (https://draculatheme.com/contribute) — 24-bit RGB palette.
//! bg=#282a36, current-line=#44475a, fg=#f8f8f2, comment=#6272a4,
//! cyan=#8be9fd, green=#50fa7b, orange=#ffb86c, pink=#ff79c6,
//! purple=#bd93f9, red=#ff5555, yellow=#f1fa8c

use ratatui::style::{Color, Modifier, Style};

use crate::theme::Theme;

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
