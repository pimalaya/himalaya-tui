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

//! Atom One Light — 24-bit RGB palette.
//! bg=#fafafa, line-bg=#e5e5e6, mono-1=#383a42, mono-3=#a0a1a7,
//! cyan=#0184bc, blue=#4078f2, magenta=#a626a4, green=#50a14f,
//! red=#e45649, orange=#c18401

use ratatui::style::{Color, Modifier, Style};

use crate::theme::Theme;

const BG: Color = Color::Rgb(0xfa, 0xfa, 0xfa);
const LINE_BG: Color = Color::Rgb(0xe5, 0xe5, 0xe6);
const MONO_1: Color = Color::Rgb(0x38, 0x3a, 0x42);
const MONO_3: Color = Color::Rgb(0xa0, 0xa1, 0xa7);
const CYAN: Color = Color::Rgb(0x01, 0x84, 0xbc);
const BLUE: Color = Color::Rgb(0x40, 0x78, 0xf2);
const ORANGE: Color = Color::Rgb(0xc1, 0x84, 0x01);

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
