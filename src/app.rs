// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026 soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify it under
// the terms of the GNU Affero General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option) any
// later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
// FOR A PARTICULAR PURPOSE. See the GNU Affero General Public License for more
// details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! State machine driving the TUI.
//!
//! [`App`] holds the panel layout, mailbox/envelope caches, current
//! dialog, compose buffer and account identity. The render functions
//! in [`crate::ui`] read it; the event loop in `main.rs` mutates it.

use clap::ValueEnum;
use edtui::{EditorEventHandler, EditorMode, EditorState, Lines};
use io_email::{envelope::Envelope, flag::Flag, mailbox::Mailbox};
use mml::template::{
    compose::builder::TemplateBuilderCompose, forward::builder::TemplateBuilderForward,
    reply::builder::TemplateBuilderReply, types::TemplateCursor,
};
use serde::{Deserialize, Serialize};

use crate::config::SmtpConfig;

/// Keybinding flavor applied to the in-app composer.
///
/// Mirrors `edtui::EditorEventHandler::{vim_mode, emacs_mode}` and is
/// shared between the CLI flag, the TOML config and the [`App`] state
/// so the event loop can also gate its own intercepts (notably
/// `Ctrl-e`, which collides with Emacs' "move to end of line").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Keybinds {
    #[default]
    Vim,
    Emacs,
}

impl Keybinds {
    /// Builds the edtui event handler that matches this flavor.
    pub fn editor_handler(self) -> EditorEventHandler {
        match self {
            Self::Vim => EditorEventHandler::vim_mode(),
            Self::Emacs => EditorEventHandler::emacs_mode(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Mailboxes,
    Envelopes,
    Message,
    Compose,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanelMode {
    None,
    Message,
    Compose,
}

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
            FlagAction::Seen => Flag::Seen,
            FlagAction::Flagged => Flag::Flagged,
            FlagAction::Answered => Flag::Answered,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    Envelope,
    Compose,
    CopyTo,
    MoveTo,
    FlagAdd,
    FlagRemove,
}

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
    pub bottom_panel_mode: BottomPanelMode,
    pub message_content: Option<String>,
    pub message_scroll: u16,
    pub previewing_compose: bool,
    pub editor_state: EditorState,
    pub editor_handler: EditorEventHandler,
    pub dialog: Option<Dialog>,
    pub dialog_index: usize,
    /// `None` means the global translation layer is inactive and only
    /// the universal keys (arrows, PageUp/Down, Tab, Esc, Enter, ...)
    /// fire. The composer still defaults to edtui's Vim handler so
    /// typing works the same way.
    pub keybinds: Option<Keybinds>,
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
            bottom_panel_mode: BottomPanelMode::None,
            message_content: None,
            message_scroll: 0,
            previewing_compose: false,
            editor_state: EditorState::new(Lines::from("")),
            editor_handler: Keybinds::default().editor_handler(),
            dialog: None,
            dialog_index: 0,
            keybinds: None,
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
            status_message: Some("Loading mailboxes...".into()),
            ..Self::default()
        }
    }

    pub fn quit(&mut self) {
        self.running = false;
    }

    pub fn close_current(&mut self) -> bool {
        match self.active_panel {
            Panel::Message | Panel::Compose => {
                self.close_bottom_panel();
                true
            }
            Panel::Envelopes => {
                if self.bottom_panel_mode != BottomPanelMode::None {
                    self.close_bottom_panel();
                } else {
                    self.unselect_mailbox();
                }
                true
            }
            _ => false,
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

    pub fn toggle_panel(&mut self) {
        self.active_panel = match self.active_panel {
            Panel::Mailboxes => Panel::Envelopes,
            Panel::Envelopes => {
                if self.bottom_panel_mode == BottomPanelMode::Message {
                    Panel::Message
                } else if self.bottom_panel_mode == BottomPanelMode::Compose {
                    Panel::Compose
                } else {
                    Panel::Mailboxes
                }
            }
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

    pub fn set_status(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    pub fn show_message(&mut self, content: String) {
        self.message_content = Some(content);
        self.message_scroll = 0;
        self.bottom_panel_mode = BottomPanelMode::Message;
        self.active_panel = Panel::Message;
    }

    pub fn close_bottom_panel(&mut self) {
        self.bottom_panel_mode = BottomPanelMode::None;
        self.message_content = None;
        self.previewing_compose = false;
        self.dialog = None;
        if self.active_panel == Panel::Message || self.active_panel == Panel::Compose {
            self.active_panel = Panel::Envelopes;
        }
    }

    pub fn preview_compose(&mut self, content: String) {
        self.message_content = Some(content);
        self.message_scroll = 0;
        self.bottom_panel_mode = BottomPanelMode::Message;
        self.active_panel = Panel::Message;
        self.previewing_compose = true;
    }

    pub fn close_preview(&mut self) {
        self.message_content = None;
        self.message_scroll = 0;
        self.previewing_compose = false;
        self.bottom_panel_mode = BottomPanelMode::Compose;
        self.active_panel = Panel::Compose;
    }

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
        state.cursor = edtui::Index2::new(cursor.row.saturating_sub(1), cursor.col);
        self.editor_state = state;
        self.bottom_panel_mode = BottomPanelMode::Compose;
        self.active_panel = Panel::Compose;
        self.dialog = None;
    }

    pub fn get_compose_content(&self) -> String {
        self.editor_state.lines.to_string()
    }

    pub fn cancel_compose(&mut self) {
        self.dialog = None;
        self.close_bottom_panel();
    }

    pub fn get_selected_envelope(&self) -> Option<&Envelope> {
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

    pub fn get_selected_envelope_action(&self) -> EnvelopeAction {
        EnvelopeAction::ALL[self.dialog_index]
    }

    pub fn get_selected_compose_action(&self) -> ComposeAction {
        ComposeAction::ALL[self.dialog_index]
    }

    pub fn get_selected_flag_action(&self) -> FlagAction {
        FlagAction::ALL[self.dialog_index]
    }
}
