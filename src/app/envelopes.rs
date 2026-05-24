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

use io_email::{envelope::Envelope, flag::Flag};

use crate::app::state::App;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeAction {
    Read,
    Reply,
    ReplyAll,
    Forward,
    Copy,
    Move,
    AddFlag,
    RemoveFlag,
}

impl EnvelopeAction {
    pub const ALL: [EnvelopeAction; 8] = [
        EnvelopeAction::Read,
        EnvelopeAction::Reply,
        EnvelopeAction::ReplyAll,
        EnvelopeAction::Forward,
        EnvelopeAction::Copy,
        EnvelopeAction::Move,
        EnvelopeAction::AddFlag,
        EnvelopeAction::RemoveFlag,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            EnvelopeAction::Read => "Read",
            EnvelopeAction::Reply => "Reply",
            EnvelopeAction::ReplyAll => "Reply All",
            EnvelopeAction::Forward => "Forward",
            EnvelopeAction::Copy => "Copy",
            EnvelopeAction::Move => "Move",
            EnvelopeAction::AddFlag => "Add flag",
            EnvelopeAction::RemoveFlag => "Remove flag",
        }
    }
}

impl App {
    pub fn set_envelopes(&mut self, envelopes: Vec<Envelope>, total: u32) {
        self.envelopes = envelopes;
        self.envelope_index = 0;
        self.envelope_offset = 0;
        self.envelope_total = total;
        self.status_message = None;
    }

    pub fn total_pages(&self) -> usize {
        if self.envelope_page_size == 0 || self.envelope_total == 0 {
            1
        } else {
            ((self.envelope_total as usize) + self.envelope_page_size - 1) / self.envelope_page_size
        }
    }

    pub fn next_envelope_page(&mut self) -> bool {
        if self.envelope_page + 1 < self.total_pages() {
            self.envelope_page += 1;
            true
        } else {
            false
        }
    }

    pub fn prev_envelope_page(&mut self) -> bool {
        if self.envelope_page > 0 {
            self.envelope_page -= 1;
            true
        } else {
            false
        }
    }

    pub fn selected_envelope(&self) -> Option<&Envelope> {
        self.envelopes.get(self.envelope_index)
    }

    pub fn remove_selected_envelope(&mut self) {
        if self.envelope_index < self.envelopes.len() {
            self.envelopes.remove(self.envelope_index);
            if self.envelope_index >= self.envelopes.len() && self.envelope_index > 0 {
                self.envelope_index -= 1;
            }
        }
    }

    pub fn flag_selected_envelope(&mut self, flag: Flag) {
        if let Some(envelope) = self.envelopes.get_mut(self.envelope_index) {
            envelope.flags.insert(flag);
        }
    }

    pub fn unflag_selected_envelope(&mut self, flag: Flag) {
        if let Some(envelope) = self.envelopes.get_mut(self.envelope_index) {
            envelope.flags.remove(&flag);
        }
    }
}
