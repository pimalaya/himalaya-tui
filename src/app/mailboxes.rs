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

use io_email::mailbox::Mailbox;

use crate::app::{panel::Panel, state::App};

impl App {
    pub fn select_mailbox(&mut self) {
        let mailbox = self.mailboxes.get(self.mailbox_index).cloned();

        if let Some(m) = mailbox {
            self.selected_mailbox = Some(m.id.clone());
            self.envelope_index = 0;
            self.envelope_offset = 0;
            self.envelope_page = 0;
            self.envelope_total = 0;
            self.envelopes.clear();
            self.close_bottom_panel();
            self.active_panel = Panel::Envelopes;
            self.status_message = Some(format!("Loading envelopes from {}...", m.name));
        }
    }

    pub fn unselect_mailbox(&mut self) {
        self.selected_mailbox = None;
        self.envelopes.clear();
        self.envelope_index = 0;
        self.envelope_offset = 0;
        self.envelope_page = 0;
        self.envelope_total = 0;
        self.close_bottom_panel();
        self.active_panel = Panel::Mailboxes;
    }

    pub fn selected_mailbox_name(&self) -> Option<&str> {
        let id = self.selected_mailbox.as_deref()?;
        self.mailboxes
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.as_str())
    }

    pub fn set_mailboxes(&mut self, mailboxes: Vec<Mailbox>) {
        self.mailboxes = mailboxes;
        self.mailbox_index = self
            .mailboxes
            .iter()
            .position(|m| m.name.eq_ignore_ascii_case("inbox"))
            .unwrap_or(0);
        if !self.mailboxes.is_empty() {
            self.select_mailbox();
        }
        self.status_message = None;
    }
}
