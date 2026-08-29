//! # Model
//!
//! Model layer of the Elm Architecture: every piece of TUI state, plus
//! the [`Message`] enum naming each transition. Mutation lives in
//! [`crate::tui::update`], rendering in [`crate::tui::view`].

use std::time::{Duration, Instant};

use clap::ValueEnum;
use edtui::{EditorEventHandler, EditorState};
use ratatui::crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use tui_input::Input;

use crate::{
    email::{
        envelope::Envelope,
        flag::{Flag, IanaFlag},
        mailbox::Mailbox,
    },
    shared::client::EmailClient,
    tui::theme::Theme,
};

/// Mailbox rows visible inside the CopyTo and MoveTo dialog list.
///
/// The view sizes the block from it and the update layer clamps the
/// selection to it.
pub const MAILBOX_DIALOG_VISIBLE: usize = 10;

/// Minimum idle period after which the app pings the storage backend.
///
/// Tuned below the tightest common submission timeout (around 120 s
/// behind corporate NAT and cloud firewalls) so the connection stays
/// warm during long reading sessions.
pub const PING_INTERVAL: Duration = Duration::from_secs(60);

/// Every piece of TUI state, including the email client.
pub struct Model {
    /// The event loop runs as long as this holds.
    pub running: bool,
    /// Panel currently receiving keys.
    pub active_panel: Panel,
    /// Mailboxes of the account, as last loaded.
    pub mailboxes: Vec<Mailbox>,
    /// Index of the highlighted mailbox.
    pub mailbox_index: usize,
    /// First mailbox row visible in the panel.
    pub mailbox_offset: usize,
    /// Filter input narrowing the mailbox list.
    pub mailbox_filter: Input,
    /// Envelopes of the loaded page.
    pub envelopes: Vec<Envelope>,
    /// Index of the highlighted envelope within the page.
    pub envelope_index: usize,
    /// First envelope row visible in the panel.
    pub envelope_offset: usize,
    /// Zero-based index of the loaded page.
    pub envelope_page: usize,
    /// Envelopes the current page was fetched with, one screenful of them.
    pub envelope_page_size: usize,
    /// Envelope rows a full-height panel can show, measured by the last
    /// render.
    ///
    /// Zero before the first render. The page size follows it, so a page
    /// is exactly what fits on screen and no page is half read.
    pub envelope_capacity: usize,
    /// Envelopes the backend reports for the selected mailbox.
    pub envelope_total: u32,
    /// Identifier of the mailbox the envelopes were loaded from.
    pub selected_mailbox: Option<String>,
    /// Name of the account the TUI runs on.
    pub account_name: String,
    /// Sender address the composer writes from.
    pub from: Option<String>,
    /// Display name paired with the sender address.
    pub from_name: Option<String>,
    /// Signature block appended verbatim by the mml template builders.
    ///
    /// Separator included, and empty when the account declares none.
    pub signature: String,
    /// Transient message shown in the status bar.
    pub status_message: Option<String>,
    /// Sub-modality of the bottom pane.
    pub bottom_panel: BottomPanel,
    /// Body of the message being read, once fetched.
    pub message_content: Option<String>,
    /// Vertical scroll offset of the message body.
    pub message_scroll: u16,
    /// Composer buffer owned by edtui.
    pub editor_state: EditorState,
    /// edtui handler translating keys for the composer.
    pub editor_handler: EditorEventHandler,
    /// Dialog open over the panels, if any.
    pub dialog: Option<Dialog>,
    /// Index of the highlighted dialog entry.
    pub dialog_index: usize,
    /// Composer keybinding flavor, Vim when unset.
    ///
    /// Affects only the in-composer edtui handler: top-level navigation
    /// always recognises both Vim and Emacs aliases.
    pub keybinds: Option<Keybinds>,
    /// Resolved colors every render function reads.
    pub theme: Theme,
    /// Backend client every network action goes through.
    pub client: EmailClient,
    /// Timestamp of the last successful network round-trip.
    ///
    /// The app loop dispatches [`Message::Ping`] once the elapsed time
    /// crosses [`PING_INTERVAL`].
    pub last_activity: Instant,
}

impl Model {
    /// Envelope the cursor is on, if the page has one.
    pub fn selected_envelope(&self) -> Option<&Envelope> {
        self.envelopes.get(self.envelope_index)
    }

    /// Display name of the mailbox the envelopes were loaded from.
    pub fn selected_mailbox_name(&self) -> Option<&str> {
        let id = self.selected_mailbox.as_deref()?;
        self.mailboxes
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.as_str())
    }

    /// Mailboxes whose name contains the filter input, case-insensitively.
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

    /// Number of entries the open dialog lists.
    pub fn dialog_item_count(&self) -> usize {
        match self.dialog {
            Some(Dialog::Envelope) => EnvelopeAction::ALL.len(),
            Some(Dialog::Compose) => ComposeAction::ALL.len(),
            Some(Dialog::CopyTo) | Some(Dialog::MoveTo) => self.filtered_mailboxes().len(),
            Some(Dialog::FlagAdd) | Some(Dialog::FlagRemove) => FlagAction::ALL.len(),
            None => 0,
        }
    }

    /// Number of pages the selected mailbox spans, at least one.
    pub fn total_pages(&self) -> usize {
        if self.envelope_page_size == 0 || self.envelope_total == 0 {
            1
        } else {
            (self.envelope_total as usize).div_ceil(self.envelope_page_size)
        }
    }

    /// Composer buffer as a string.
    pub fn compose_content(&self) -> String {
        self.editor_state.lines.to_string()
    }

    /// Envelope action the dialog cursor is on.
    pub fn selected_envelope_action(&self) -> EnvelopeAction {
        EnvelopeAction::ALL[self.dialog_index]
    }

    /// Compose action the dialog cursor is on.
    pub fn selected_compose_action(&self) -> ComposeAction {
        ComposeAction::ALL[self.dialog_index]
    }

    /// Flag action the dialog cursor is on.
    pub fn selected_flag_action(&self) -> FlagAction {
        FlagAction::ALL[self.dialog_index]
    }
}

/// Active focus among the four panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    /// Mailbox list.
    Mailboxes,
    /// Envelope list of the selected mailbox.
    Envelopes,
    /// Body of the message being read.
    Message,
    /// Composer buffer.
    Compose,
}

/// Sub-modality of the bottom pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottomPanel {
    /// Nothing shown, the pane is closed.
    None,
    /// A stored message being read.
    Message,
    /// Compiled MIME of an in-flight compose buffer.
    ///
    /// Esc returns to the composer instead of closing the pane.
    MessagePreview,
    /// The composer itself.
    Compose,
}

/// Dialog open over the panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    /// Actions available on the selected envelope.
    Envelope,
    /// Actions available on the compose buffer.
    Compose,
    /// Mailbox picker for a copy.
    CopyTo,
    /// Mailbox picker for a move.
    MoveTo,
    /// Flag picker for an addition.
    FlagAdd,
    /// Flag picker for a removal.
    FlagRemove,
}

/// Action offered by the envelope dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeAction {
    /// Open the message in the bottom pane.
    Read,
    /// Compose a reply to the sender.
    Reply,
    /// Compose a reply to every recipient.
    ReplyAll,
    /// Compose a forward of the message.
    Forward,
    /// Copy the message to another mailbox.
    Copy,
    /// Move the message to another mailbox.
    Move,
    /// Add a flag to the message.
    AddFlag,
    /// Remove a flag from the message.
    RemoveFlag,
}

impl EnvelopeAction {
    /// Every envelope action, in dialog order.
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

    /// Label shown in the dialog.
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

/// Action offered by the compose dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeAction {
    /// Send the buffer through the backend.
    Send,
    /// Show the compiled MIME in the bottom pane.
    Preview,
    /// Save the buffer to the drafts mailbox.
    SaveToDrafts,
    /// Discard the buffer and leave the composer.
    Cancel,
}

impl ComposeAction {
    /// Every compose action, in dialog order.
    pub const ALL: [ComposeAction; 4] = [
        ComposeAction::Send,
        ComposeAction::Preview,
        ComposeAction::SaveToDrafts,
        ComposeAction::Cancel,
    ];

    /// Label shown in the dialog.
    pub fn label(&self) -> &'static str {
        match self {
            ComposeAction::Send => "Send",
            ComposeAction::Preview => "Preview",
            ComposeAction::SaveToDrafts => "Save to Drafts",
            ComposeAction::Cancel => "Cancel",
        }
    }
}

/// Flag offered by the add and remove dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagAction {
    /// The seen flag.
    Seen,
    /// The flagged flag.
    Flagged,
    /// The answered flag.
    Answered,
}

impl FlagAction {
    /// Every flag action, in dialog order.
    pub const ALL: [FlagAction; 3] = [FlagAction::Seen, FlagAction::Flagged, FlagAction::Answered];

    /// Label shown in the dialog.
    pub fn label(&self) -> &'static str {
        match self {
            FlagAction::Seen => "Seen",
            FlagAction::Flagged => "Flagged",
            FlagAction::Answered => "Answered",
        }
    }

    /// Matching IANA flag.
    pub fn flag(&self) -> Flag {
        match self {
            FlagAction::Seen => Flag::from_iana(IanaFlag::Seen),
            FlagAction::Flagged => Flag::from_iana(IanaFlag::Flagged),
            FlagAction::Answered => Flag::from_iana(IanaFlag::Answered),
        }
    }
}

/// Composer keybinding flavor.
///
/// Shared between the CLI flag, the TOML config and the [`Model`], and
/// mirrors `edtui::EditorEventHandler::{vim_mode, emacs_mode}`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Keybinds {
    /// Modal Vim bindings, the default.
    #[default]
    Vim,
    /// Emacs bindings.
    Emacs,
}

impl Keybinds {
    /// edtui event handler implementing this flavor.
    pub fn editor_handler(self) -> EditorEventHandler {
        match self {
            Self::Vim => EditorEventHandler::vim_mode(),
            Self::Emacs => EditorEventHandler::emacs_mode(),
        }
    }
}

/// Which end of a freshly loaded page the selection lands on.
///
/// A page reached by moving down is entered from its top, one reached
/// by moving up from its bottom, so a cursor crossing the boundary
/// keeps travelling in the direction it was going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeLanding {
    /// Select the first envelope of the page.
    First,
    /// Select the last envelope of the page.
    Last,
    /// Select the envelope at this offset into the page.
    ///
    /// Clamped to the last one. Used when a resized page has to keep
    /// the cursor on the envelope it was on.
    Index(usize),
}

/// Every state transition is named here.
///
/// Raw keys enter as [`Message::Key`] and are translated to domain
/// variants inside [`crate::tui::update`].
#[derive(Debug, Clone)]
pub enum Message {
    /// A key press, before any translation.
    Key(KeyEvent),
    /// Stop the event loop.
    Quit,
    /// First transition, dispatched before anything is loaded.
    Initialize,
    /// Move the focus to the next panel.
    TogglePanel,
    /// Move the selection one step down.
    Next,
    /// Move the selection one step up.
    Previous,
    /// Move the selection one page down.
    PageDown,
    /// Move the selection one page up.
    PageUp,
    /// Activate the focused panel's selection.
    Enter,
    /// Dismiss the dialog, or close the bottom pane.
    Esc,
    /// Open the composer on a blank message.
    StartCompose,
    /// A key press routed to the composer.
    EditorKey(KeyEvent),
    /// Hand the compose buffer over to the system editor.
    OpenSystemEditor,
    /// A key press routed to the mailbox filter input.
    MailboxFilterKey(KeyEvent),
    /// Move the dialog cursor down.
    DialogNext,
    /// Move the dialog cursor up.
    DialogPrevious,
    /// Run the highlighted dialog action.
    DialogConfirm,
    /// Close the dialog without acting.
    DialogClose,
    /// Keep the backend connection warm after an idle period.
    Ping,
    /// Fetch the mailboxes of the account.
    LoadMailboxes,
    /// Fetch the current envelope page and land the cursor.
    LoadEnvelopes(EnvelopeLanding),
    /// Fetch the selected message and show it.
    ReadSelected,
    /// Open the composer on a reply to the selected message.
    StartReplyToSelected { reply_all: bool },
    /// Open the composer on a forward of the selected message.
    StartForwardSelected,
    /// Copy the selected message to the picked mailbox.
    CopySelectedToTarget,
    /// Move the selected message to the picked mailbox.
    MoveSelectedToTarget,
    /// Add or remove the picked flag on the selected message.
    FlagSelected { add: bool },
    /// Send the compose buffer.
    SendCompose,
    /// Compile the compose buffer and show the result.
    PreviewCompose,
    /// Save the compose buffer to the drafts mailbox.
    SaveComposeToDrafts,
    /// Discard the compose buffer.
    CancelCompose,
}
