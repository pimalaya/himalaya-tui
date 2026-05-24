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

use io_email::flag::{Flag, IanaFlag};

use crate::app::{compose::ComposeAction, envelopes::EnvelopeAction, state::App};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    Envelope,
    Compose,
    CopyTo,
    MoveTo,
    FlagAdd,
    FlagRemove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagAction {
    Seen,
    Flagged,
    Answered,
}

impl FlagAction {
    pub const ALL: [FlagAction; 3] = [FlagAction::Seen, FlagAction::Flagged, FlagAction::Answered];

    pub fn label(&self) -> &'static str {
        match self {
            FlagAction::Seen => "Seen",
            FlagAction::Flagged => "Flagged",
            FlagAction::Answered => "Answered",
        }
    }

    pub fn flag(&self) -> Flag {
        match self {
            FlagAction::Seen => Flag::from_iana(IanaFlag::Seen),
            FlagAction::Flagged => Flag::from_iana(IanaFlag::Flagged),
            FlagAction::Answered => Flag::from_iana(IanaFlag::Answered),
        }
    }
}

impl App {
    pub fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
        self.dialog_index = 0;
    }

    pub fn close_dialog(&mut self) {
        self.dialog = None;
    }

    pub fn dialog_item_count(&self) -> usize {
        match self.dialog {
            Some(Dialog::Envelope) => EnvelopeAction::ALL.len(),
            Some(Dialog::Compose) => ComposeAction::ALL.len(),
            Some(Dialog::CopyTo) | Some(Dialog::MoveTo) => self.mailboxes.len(),
            Some(Dialog::FlagAdd) | Some(Dialog::FlagRemove) => FlagAction::ALL.len(),
            None => 0,
        }
    }

    pub fn dialog_next(&mut self) {
        let count = self.dialog_item_count();
        if count > 0 {
            self.dialog_index = (self.dialog_index + 1) % count;
        }
    }

    pub fn dialog_previous(&mut self) {
        let count = self.dialog_item_count();
        if count > 0 {
            self.dialog_index = self.dialog_index.checked_sub(1).unwrap_or(count - 1);
        }
    }

    pub fn selected_envelope_action(&self) -> EnvelopeAction {
        EnvelopeAction::ALL[self.dialog_index]
    }

    pub fn selected_compose_action(&self) -> ComposeAction {
        ComposeAction::ALL[self.dialog_index]
    }

    pub fn selected_flag_action(&self) -> FlagAction {
        FlagAction::ALL[self.dialog_index]
    }
}
