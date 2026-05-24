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

//! Resolved [`Theme`] used by every render function. Each themable
//! element is a ratatui [`Style`], so the same place tunes background,
//! foreground, and modifiers (bold/italic/...). Themes are plain
//! `const` values defined in Rust files under `src/themes/`; the
//! built-in default uses named ANSI colors so the UI inherits the
//! user's terminal palette.
//!
//! User configuration (`ThemeConfig` in [`crate::config`]) layers
//! per-field overrides on top of the chosen preset via
//! [`Style::patch`].

use ratatui::style::Style;

use crate::{
    config::{PresetConfig, ThemeConfig},
    themes,
};

/// Resolved theme used by every render function.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub header: Style,
    pub status_bar: Style,
    pub border_active: Style,
    pub border_inactive: Style,
    pub dialog_border: Style,
    pub cursor: Style,
    pub mailbox_current: Style,
    pub envelope_header: Style,
    pub envelope_seen: Style,
    pub envelope_unread: Style,
    pub message_body: Style,
    pub compose_text: Style,
    pub compose_cursor: Style,
    pub compose_selection: Style,
}

impl Default for Theme {
    fn default() -> Self {
        themes::default::THEME
    }
}

impl Theme {
    /// Starts from the preset (or the built-in default), then layers
    /// per-field overrides on top using [`Style::patch`] so partial
    /// overrides (e.g. only `fg`) keep untouched fields from the
    /// preset.
    pub fn resolve(config: &ThemeConfig) -> Self {
        let mut t = config.preset.unwrap_or(PresetConfig::Default).theme();

        if let Some(s) = &config.header {
            t.header = t.header.patch(Style::from(s));
        }

        if let Some(s) = &config.status_bar {
            t.status_bar = t.status_bar.patch(Style::from(s));
        }

        if let Some(s) = &config.border_active {
            t.border_active = t.border_active.patch(Style::from(s));
        }

        if let Some(s) = &config.border_inactive {
            t.border_inactive = t.border_inactive.patch(Style::from(s));
        }

        if let Some(s) = &config.dialog_border {
            t.dialog_border = t.dialog_border.patch(Style::from(s));
        }

        if let Some(s) = &config.cursor {
            t.cursor = t.cursor.patch(Style::from(s));
        }

        if let Some(s) = &config.mailbox_current {
            t.mailbox_current = t.mailbox_current.patch(Style::from(s));
        }

        if let Some(s) = &config.envelope_header {
            t.envelope_header = t.envelope_header.patch(Style::from(s));
        }

        if let Some(s) = &config.envelope_seen {
            t.envelope_seen = t.envelope_seen.patch(Style::from(s));
        }

        if let Some(s) = &config.envelope_unread {
            t.envelope_unread = t.envelope_unread.patch(Style::from(s));
        }

        if let Some(s) = &config.message_body {
            t.message_body = t.message_body.patch(Style::from(s));
        }

        if let Some(s) = &config.compose_text {
            t.compose_text = t.compose_text.patch(Style::from(s));
        }

        if let Some(s) = &config.compose_cursor {
            t.compose_cursor = t.compose_cursor.patch(Style::from(s));
        }

        if let Some(s) = &config.compose_selection {
            t.compose_selection = t.compose_selection.patch(Style::from(s));
        }

        t
    }
}
