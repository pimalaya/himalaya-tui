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

//! Top-level state container for the TUI.
//!
//! [`App`] owns every piece of mutable state the render and event-loop
//! modules read or write: panel layout, mailbox/envelope caches,
//! dialog, compose buffer, account identity, theme. Behavior is split
//! across the sibling submodules under `crate::app`.

use edtui::{EditorEventHandler, EditorState, Lines};
use io_email::{envelope::Envelope, mailbox::Mailbox};

use crate::{
    app::{dialog::Dialog, keybinds::Keybinds, panel::BottomPanel, panel::Panel},
    config::SmtpConfig,
    theme::Theme,
};

pub struct App {
    pub running: bool,
    pub active_panel: Panel,
    pub mailboxes: Vec<Mailbox>,
    pub mailbox_index: usize,
    pub mailbox_offset: usize,
    pub envelopes: Vec<Envelope>,
    pub envelope_index: usize,
    pub envelope_offset: usize,
    pub envelope_page: usize,
    pub envelope_page_size: usize,
    pub envelope_total: u32,
    pub selected_mailbox: Option<String>,
    pub account_name: String,
    pub from: Option<String>,
    pub from_name: Option<String>,
    pub signature: String,
    pub smtp_config: Option<SmtpConfig>,
    pub status_message: Option<String>,
    pub bottom_panel: BottomPanel,
    pub message_content: Option<String>,
    pub message_scroll: u16,
    pub editor_state: EditorState,
    pub editor_handler: EditorEventHandler,
    pub dialog: Option<Dialog>,
    pub dialog_index: usize,
    /// Composer flavor. Drives only edtui's event handler (Vim vs
    /// Emacs insert/normal-mode bindings). Top-level navigation is
    /// independent: Vim and Emacs aliases are always merged on top of
    /// the universal keys (arrows, PageUp/Down, Tab, Esc, Enter, ...).
    /// `None` falls back to the composer's default Vim handler.
    pub keybinds: Option<Keybinds>,
    pub theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            active_panel: Panel::Mailboxes,
            mailboxes: Vec::new(),
            mailbox_index: 0,
            mailbox_offset: 0,
            envelopes: Vec::new(),
            envelope_index: 0,
            envelope_offset: 0,
            envelope_page: 0,
            envelope_page_size: 50,
            envelope_total: 0,
            selected_mailbox: None,
            account_name: String::new(),
            from: None,
            from_name: None,
            signature: String::new(),
            smtp_config: None,
            status_message: None,
            bottom_panel: BottomPanel::None,
            message_content: None,
            message_scroll: 0,
            editor_state: EditorState::new(Lines::from("")),
            editor_handler: Keybinds::default().editor_handler(),
            dialog: None,
            dialog_index: 0,
            keybinds: None,
            theme: Theme::default(),
        }
    }
}

impl App {
    pub fn new(
        account_name: String,
        from: Option<String>,
        from_name: Option<String>,
        signature: String,
        smtp_config: Option<SmtpConfig>,
        keybinds: Option<Keybinds>,
        theme: Theme,
    ) -> Self {
        // edtui can only be Vim or Emacs, so map the absent case to
        // its default (Vim) for the composer.
        let editor_handler = keybinds.unwrap_or_default().editor_handler();
        Self {
            account_name,
            from,
            from_name,
            signature,
            smtp_config,
            editor_handler,
            keybinds,
            theme,
            ..Self::default()
        }
    }
}
