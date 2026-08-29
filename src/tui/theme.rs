//! # Theme
//!
//! Color themes for the TUI. [`Theme`] is what every render function
//! reads: one ratatui [`Style`] per themable element, so background,
//! foreground and modifiers are tuned in one place.
//!
//! The built-in presets are plain const values, one per submodule.
//! [`Theme::resolve`] layers the per-field overrides coming from
//! [`crate::config::ThemeConfig`] on top of the chosen preset.

pub mod default;
pub mod dracula_dark;
pub mod one_light;
pub mod tokyo_night;

use ratatui::style::Style;

use crate::{
    config::{PresetConfig, ThemeConfig},
    tui::theme,
};

/// Resolved theme used by every render function.
#[derive(Clone, Copy, Debug)]
pub struct Theme {
    /// Title bar at the top of the screen.
    pub header: Style,
    /// Status bar at the bottom of the screen.
    pub status_bar: Style,
    /// Border of the focused panel.
    pub border_active: Style,
    /// Border of the unfocused panels.
    pub border_inactive: Style,
    /// Border of a dialog.
    pub dialog_border: Style,
    /// Row the selection sits on.
    pub cursor: Style,
    /// Mailbox the envelopes were loaded from.
    pub mailbox_current: Style,
    /// Column headers of the envelope table.
    pub envelope_header: Style,
    /// Envelope row already seen.
    pub envelope_seen: Style,
    /// Envelope row still unread.
    pub envelope_unread: Style,
    /// Body of the message being read.
    pub message_body: Style,
    /// Text of the compose buffer.
    pub compose_text: Style,
    /// Cursor inside the compose buffer.
    pub compose_cursor: Style,
    /// Selected text inside the compose buffer.
    pub compose_selection: Style,
}

impl Default for Theme {
    fn default() -> Self {
        theme::default::THEME
    }
}

impl Theme {
    /// Resolves a theme from its preset and per-field overrides.
    ///
    /// Starts from the configured preset, or the built-in default, then
    /// layers each override with [`Style::patch`] so a partial override
    /// (only `fg`, say) keeps the untouched fields of the preset.
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
