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

//! Model layer of the Elm Architecture: every piece of TUI state plus
//! the [`Message`] enum naming each transition. Mutation happens in
//! [`crate::tui::update`]; rendering in [`crate::tui::view`].

use std::time::{Duration, Instant};

use clap::ValueEnum;
use edtui::{EditorEventHandler, EditorState};
use io_email::{
    client::EmailClientStd,
    envelope::Envelope,
    flag::{Flag, IanaFlag},
    mailbox::Mailbox,
};
use ratatui::crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use tui_input::Input;

use crate::tui::theme::Theme;

/// Number of mailbox rows visible inside the CopyTo/MoveTo dialog
/// list block. Both the view (frame sizing) and the update layer
/// (selection clamping) depend on this constant.
pub const MAILBOX_DIALOG_VISIBLE: usize = 10;

/// Minimum idle period after which the app issues a NOOP to every
/// registered network backend. Tuned below the tightest common SMTP
/// submission timeout (~120 s behind corporate NAT / cloud firewalls)
/// so connections stay warm during long reading sessions.
pub const PING_INTERVAL: Duration = Duration::from_secs(60);

pub struct Model {
    pub running: bool,
    pub active_panel: Panel,
    pub mailboxes: Vec<Mailbox>,
    pub mailbox_index: usize,
    pub mailbox_offset: usize,
    pub mailbox_filter: Input,
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
    pub status_message: Option<String>,
    pub bottom_panel: BottomPanel,
    pub message_content: Option<String>,
    pub message_scroll: u16,
    pub editor_state: EditorState,
    pub editor_handler: EditorEventHandler,
    pub dialog: Option<Dialog>,
    pub dialog_index: usize,
    /// Composer flavor. Affects only the in-composer edtui handler;
    /// top-level navigation always recognises both Vim and Emacs
    /// aliases. `None` falls back to Vim.
    pub keybinds: Option<Keybinds>,
    pub theme: Theme,
    pub client: EmailClientStd,
    /// Timestamp of the last successful network round-trip (any user
    /// action or NOOP). The app loop dispatches [`Message::Ping`] once
    /// the elapsed time crosses [`PING_INTERVAL`].
    pub last_activity: Instant,
}

impl Model {
    pub fn selected_envelope(&self) -> Option<&Envelope> {
        self.envelopes.get(self.envelope_index)
    }

    pub fn selected_mailbox_name(&self) -> Option<&str> {
        let id = self.selected_mailbox.as_deref()?;
        self.mailboxes
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.as_str())
    }

    /// Mailboxes visible after applying the filter input.
    /// Case-insensitive substring match on the name; empty filter
    /// returns the full list.
    pub fn filtered_mailboxes(&self) -> Vec<&Mailbox> {
        let needle = self.mailbox_filter.value();
        if needle.is_empty() {
            return self.mailboxes.iter().collect();
        }

        let needle = needle.to_lowercase();

        // TODO: improve the search algorithm
        self.mailboxes
            .iter()
            .filter(|m| m.name.to_lowercase().contains(&needle))
            .collect()
    }

    pub fn dialog_item_count(&self) -> usize {
        match self.dialog {
            Some(Dialog::Envelope) => EnvelopeAction::ALL.len(),
            Some(Dialog::Compose) => ComposeAction::ALL.len(),
            Some(Dialog::CopyTo) | Some(Dialog::MoveTo) => self.filtered_mailboxes().len(),
            Some(Dialog::FlagAdd) | Some(Dialog::FlagRemove) => FlagAction::ALL.len(),
            None => 0,
        }
    }

    pub fn total_pages(&self) -> usize {
        if self.envelope_page_size == 0 || self.envelope_total == 0 {
            1
        } else {
            ((self.envelope_total as usize) + self.envelope_page_size - 1) / self.envelope_page_size
        }
    }

    pub fn compose_content(&self) -> String {
        self.editor_state.lines.to_string()
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

/// Active focus among the four panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Mailboxes,
    Envelopes,
    Message,
    Compose,
}

/// Sub-modality of the right-hand pane. `MessagePreview` is the
/// compiled MIME of an in-flight compose buffer (Esc returns to the
/// composer instead of closing), as opposed to `Message` which is a
/// stored message being read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanel {
    None,
    Message,
    MessagePreview,
    Compose,
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

/// Composer keybinding flavor. Shared between the CLI flag, the TOML
/// config and the [`Model`]; mirrors
/// `edtui::EditorEventHandler::{vim_mode, emacs_mode}`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Keybinds {
    #[default]
    Vim,
    Emacs,
}

impl Keybinds {
    pub fn editor_handler(self) -> EditorEventHandler {
        match self {
            Self::Vim => EditorEventHandler::vim_mode(),
            Self::Emacs => EditorEventHandler::emacs_mode(),
        }
    }
}

/// Every state transition is named here. Raw keys enter as
/// [`Message::Key`] and are translated to domain variants inside
/// [`crate::tui::update`].
#[derive(Debug, Clone)]
pub enum Message {
    Key(KeyEvent),

    Quit,
    Initialize,

    TogglePanel,
    Next,
    Previous,
    PageDown,
    PageUp,
    Enter,
    Esc,
    StartCompose,

    EditorKey(KeyEvent),
    OpenSystemEditor,

    MailboxFilterKey(KeyEvent),

    DialogNext,
    DialogPrevious,
    DialogConfirm,
    DialogClose,

    Ping,

    LoadMailboxes,
    LoadEnvelopes,
    ReadSelectedMessage,
    StartReplyToSelected { reply_all: bool },
    StartForwardSelected,
    CopySelectedToTarget,
    MoveSelectedToTarget,
    FlagSelected { add: bool },
    SendCompose,
    PreviewCompose,
    SaveComposeToDrafts,
    CancelCompose,
}
