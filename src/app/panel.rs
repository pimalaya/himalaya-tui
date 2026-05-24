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

use crate::app::state::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Mailboxes,
    Envelopes,
    Message,
    Compose,
}

/// Sub-modality of the right-hand pane. `Message` is a stored
/// message being read; `MessagePreview` is the compiled MIME of the
/// in-flight compose buffer (Esc returns to the composer instead of
/// closing); `Compose` is the composer itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanel {
    None,
    Message,
    MessagePreview,
    Compose,
}

impl App {
    pub fn close_current(&mut self) -> bool {
        match self.active_panel {
            Panel::Message | Panel::Compose => {
                self.close_bottom_panel();
                true
            }
            Panel::Envelopes => {
                if self.bottom_panel != BottomPanel::None {
                    self.close_bottom_panel();
                } else {
                    self.unselect_mailbox();
                }
                true
            }
            _ => false,
        }
    }

    pub fn toggle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Mailboxes => Panel::Envelopes,
            Panel::Envelopes => match self.bottom_panel {
                BottomPanel::Message | BottomPanel::MessagePreview => Panel::Message,
                BottomPanel::Compose => Panel::Compose,
                BottomPanel::None => Panel::Mailboxes,
            },
            Panel::Message => Panel::Mailboxes,
            Panel::Compose => Panel::Mailboxes,
        };
    }

    pub fn next_item(&mut self) {
        match self.active_panel {
            Panel::Mailboxes => {
                if self.mailbox_index + 1 < self.mailboxes.len() {
                    self.mailbox_index += 1;
                }
            }
            Panel::Envelopes => {
                if self.envelope_index + 1 < self.envelopes.len() {
                    self.envelope_index += 1;
                }
            }
            Panel::Message => {
                self.message_scroll = self.message_scroll.saturating_add(1);
            }
            Panel::Compose => {}
        }
    }

    pub fn previous_item(&mut self) {
        match self.active_panel {
            Panel::Mailboxes => {
                self.mailbox_index = self.mailbox_index.saturating_sub(1);
            }
            Panel::Envelopes => {
                self.envelope_index = self.envelope_index.saturating_sub(1);
            }
            Panel::Message => {
                self.message_scroll = self.message_scroll.saturating_sub(1);
            }
            Panel::Compose => {}
        }
    }
}
