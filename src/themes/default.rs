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

//! Built-in default theme using named ANSI colors. Lets the user's
//! terminal palette decide the actual shade, so the TUI blends with
//! whatever color scheme they already use.

use ratatui::style::{Color, Modifier, Style};

use crate::theme::Theme;

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
