//! Model layer of the Elm Architecture: every piece of TUI state plus
//! the [`Message`] enum naming each transition. Mutation happens in
//! [`crate::tui::update`]; rendering in [`crate::tui::view`].

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
    tui::{
        command::{self, CommandContext},
        palette::{PaletteMessage, PaletteState},
        theme::Theme,
    },
};

/// Number of mailbox rows visible inside the CopyTo/MoveTo dialog
/// list block. Both the view (frame sizing) and the update layer
/// (selection clamping) depend on this constant.
pub const MAILBOX_DIALOG_VISIBLE: usize = 10;

/// Minimum idle period after which the app issues a NOOP to every
/// registered network backend. Tuned below the tightest common SMTP
/// submission timeout (~120 s behind corporate NAT / cloud firewalls)
/// so connections stay warm during long reading sessions.
pub const PING_INTERVAL: Duration = Duration::from_secs(60);

pub const DEFAULT_ENVELOPE_PAGE_SIZE: usize = 50;

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
    pub palette: Option<PaletteState>,
    /// Keybinding flavor: selects the in-composer edtui handler and
    /// which flavor-scoped command keys are active. Resolved from
    /// CLI/config at startup; list-navigation aliases stay
    /// flavor-neutral.
    pub keybinds: Keybinds,
    pub palette_key: char,
    pub theme: Theme,
    pub client: EmailClient,
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
        let Some(dialog) = self.dialog else { return 0 };
        if let Some(context) = dialog.command_context() {
            return command::for_context(context).count();
        }
        match dialog {
            Dialog::CopyTo | Dialog::MoveTo => self.filtered_mailboxes().len(),
            _ => FlagAction::ALL.len(),
        }
    }

    pub fn total_pages(&self) -> usize {
        if self.envelope_page_size == 0 || self.envelope_total == 0 {
            1
        } else {
            (self.envelope_total as usize).div_ceil(self.envelope_page_size)
        }
    }

    pub fn compose_content(&self) -> String {
        self.editor_state.lines.to_string()
    }

    pub fn selected_flag_action(&self) -> FlagAction {
        FlagAction::ALL[self.dialog_index]
    }
}

impl Model {
    /// A blank, running model over the given client; callers layer
    /// their own fields on via struct-update syntax.
    pub fn new(client: EmailClient) -> Self {
        Self {
            client,
            running: true,
            active_panel: Panel::Mailboxes,
            mailboxes: Vec::new(),
            mailbox_index: 0,
            mailbox_offset: 0,
            mailbox_filter: Input::default(),
            envelopes: Vec::new(),
            envelope_index: 0,
            envelope_offset: 0,
            envelope_page: 0,
            envelope_page_size: DEFAULT_ENVELOPE_PAGE_SIZE,
            envelope_total: 0,
            selected_mailbox: None,
            account_name: String::new(),
            from: None,
            from_name: None,
            signature: String::new(),
            status_message: None,
            bottom_panel: BottomPanel::None,
            message_content: None,
            message_scroll: 0,
            editor_state: EditorState::default(),
            editor_handler: Keybinds::default().editor_handler(),
            dialog: None,
            dialog_index: 0,
            palette: None,
            keybinds: Keybinds::default(),
            palette_key: crate::config::DEFAULT_PALETTE_KEY,
            theme: Theme::default(),
            last_activity: Instant::now(),
        }
    }
}

/// Backend-less model for tests; the client can never perform I/O.
#[cfg(test)]
impl Default for Model {
    fn default() -> Self {
        Self::new(EmailClient::default())
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

impl Dialog {
    /// The registry context backing this dialog's rows; `None` for
    /// the parameter pickers (mailbox and flag dialogs).
    pub fn command_context(self) -> Option<CommandContext> {
        match self {
            Dialog::Envelope => Some(CommandContext::Envelope),
            Dialog::Compose => Some(CommandContext::Composer),
            Dialog::CopyTo | Dialog::MoveTo | Dialog::FlagAdd | Dialog::FlagRemove => None,
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

    Palette(PaletteMessage),

    OpenDialog(Dialog),
    DialogNext,
    DialogPrevious,
    DialogConfirm,
    DialogClose,

    Ping,

    LoadMailboxes,
    LoadEnvelopes,
    ReadSelected,
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
