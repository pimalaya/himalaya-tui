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

use edtui::{EditorMode, EditorState, Index2, Lines};
use mml::template::{
    compose::builder::TemplateBuilderCompose, forward::builder::TemplateBuilderForward,
    reply::builder::TemplateBuilderReply, types::TemplateCursor,
};

use crate::app::{
    panel::{BottomPanel, Panel},
    state::App,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeAction {
    Send,
    Preview,
    SaveToDrafts,
    Cancel,
}

impl ComposeAction {
    pub const ALL: [ComposeAction; 4] = [
        ComposeAction::Send,
        ComposeAction::Preview,
        ComposeAction::SaveToDrafts,
        ComposeAction::Cancel,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            ComposeAction::Send => "Send",
            ComposeAction::Preview => "Preview",
            ComposeAction::SaveToDrafts => "Save to Drafts",
            ComposeAction::Cancel => "Cancel",
        }
    }
}

impl App {
    pub fn start_compose(&mut self) {
        let tpl = TemplateBuilderCompose {
            from: self.from.clone().unwrap_or_default(),
            from_name: self.from_name.clone(),
            signature: self.signature.clone(),
            ..Default::default()
        }
        .build();

        match tpl {
            Ok(tpl) => self.open_editor_with_template(&tpl.content, &tpl.cursor),
            Err(err) => self.set_status(format!("Error building template: {err}")),
        }
    }

    pub fn start_reply(&mut self, raw_message: &[u8], reply_all: bool) {
        let Some(msg) = mail_parser::MessageParser::default().parse(raw_message) else {
            self.set_status("Error: failed to parse message");
            return;
        };

        let tpl = TemplateBuilderReply {
            from: self.from.clone().unwrap_or_default(),
            from_name: self.from_name.clone(),
            signature: self.signature.clone(),
            reply_all,
            ..Default::default()
        }
        .build(&msg);

        match tpl {
            Ok(tpl) => self.open_editor_with_template(&tpl.content, &tpl.cursor),
            Err(err) => self.set_status(format!("Error building reply template: {err}")),
        }
    }

    pub fn start_forward(&mut self, raw_message: &[u8]) {
        let Some(msg) = mail_parser::MessageParser::default().parse(raw_message) else {
            self.set_status("Error: failed to parse message");
            return;
        };

        let tpl = TemplateBuilderForward {
            from: self.from.clone().unwrap_or_default(),
            from_name: self.from_name.clone(),
            signature: self.signature.clone(),
            ..Default::default()
        }
        .build(&msg);

        match tpl {
            Ok(tpl) => self.open_editor_with_template(&tpl.content, &tpl.cursor),
            Err(err) => self.set_status(format!("Error building forward template: {err}")),
        }
    }

    fn open_editor_with_template(&mut self, content: &str, cursor: &TemplateCursor) {
        let mut state = EditorState::new(Lines::from(content));
        state.mode = EditorMode::Insert;
        state.cursor = Index2::new(cursor.row.saturating_sub(1), cursor.col);
        self.editor_state = state;
        self.bottom_panel = BottomPanel::Compose;
        self.active_panel = Panel::Compose;
        self.dialog = None;
    }

    pub fn compose_content(&self) -> String {
        self.editor_state.lines.to_string()
    }

    pub fn cancel_compose(&mut self) {
        self.dialog = None;
        self.close_bottom_panel();
    }
}
