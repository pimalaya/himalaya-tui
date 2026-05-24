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

use crate::app::{
    panel::{BottomPanel, Panel},
    state::App,
};

impl App {
    pub fn show_message(&mut self, content: String) {
        self.message_content = Some(content);
        self.message_scroll = 0;
        self.bottom_panel = BottomPanel::Message;
        self.active_panel = Panel::Message;
    }

    pub fn close_bottom_panel(&mut self) {
        self.bottom_panel = BottomPanel::None;
        self.message_content = None;
        self.dialog = None;
        if self.active_panel == Panel::Message || self.active_panel == Panel::Compose {
            self.active_panel = Panel::Envelopes;
        }
    }

    pub fn preview_compose(&mut self, content: String) {
        self.message_content = Some(content);
        self.message_scroll = 0;
        self.bottom_panel = BottomPanel::MessagePreview;
        self.active_panel = Panel::Message;
    }

    pub fn close_preview(&mut self) {
        self.message_content = None;
        self.message_scroll = 0;
        self.bottom_panel = BottomPanel::Compose;
        self.active_panel = Panel::Compose;
    }
}
