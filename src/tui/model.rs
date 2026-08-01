//! Model layer of the Elm Architecture: every piece of TUI state plus
//! the [`Message`] enum naming each transition. Mutation happens in
//! [`crate::tui::update`]; rendering in [`crate::tui::view`].

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

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
    pub envelope_total: Option<u64>,
    /// Ids of multi-selected envelopes. Always a subset of the visible
    /// page: every envelope-list reload clears it.
    pub selected_envelope_ids: HashSet<String>,
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

    pub fn is_selected(&self, envelope: &Envelope) -> bool {
        self.selected_envelope_ids.contains(&envelope.id)
    }

    pub fn has_selection(&self) -> bool {
        !self.selected_envelope_ids.is_empty()
    }

    pub fn selection_count(&self) -> usize {
        self.selected_envelope_ids.len()
    }

    /// Length of [`Self::bulk_target_ids`] without allocating it;
    /// safe per the subset invariant on `selected_envelope_ids`.
    pub fn bulk_target_count(&self) -> usize {
        if self.has_selection() {
            self.selection_count()
        } else {
            usize::from(self.selected_envelope().is_some())
        }
    }

    /// Ids a bulk operation targets: the selection when one exists
    /// (in visible page order), otherwise the hovered envelope.
    pub fn bulk_target_ids(&self) -> Vec<String> {
        if self.has_selection() {
            self.envelopes
                .iter()
                .filter(|e| self.is_selected(e))
                .map(|e| e.id.clone())
                .collect()
        } else {
            self.selected_envelope()
                .map(|e| e.id.clone())
                .into_iter()
                .collect()
        }
    }

    pub fn mailbox_name(&self, id: &str) -> Option<&str> {
        self.mailboxes
            .iter()
            .find(|m| m.id == id)
            .map(|m| m.name.as_str())
    }

    pub fn selected_mailbox_name(&self) -> Option<&str> {
        self.mailbox_name(self.selected_mailbox.as_deref()?)
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

    pub fn total_pages(&self) -> Option<usize> {
        total_pages(self.envelope_total, self.envelope_page_size)
    }

    pub fn can_advance_envelope_page(&self) -> bool {
        let is_full_page = self.envelopes.len() == self.envelope_page_size;
        can_advance_page(self.envelope_page, self.total_pages(), is_full_page)
    }

    pub fn compose_content(&self) -> String {
        self.editor_state.lines.to_string()
    }

    pub fn selected_flag_action(&self) -> FlagAction {
        FlagAction::ALL[self.dialog_index]
    }
}

pub fn format_message_count(count: usize) -> String {
    if count == 1 {
        "1 message".to_string()
    } else {
        format!("{count} messages")
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
            envelope_total: None,
            selected_envelope_ids: HashSet::new(),
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

fn total_pages(total: Option<u64>, page_size: usize) -> Option<usize> {
    let total = total?;
    if page_size == 0 {
        return Some(1);
    }
    Some((total as usize).div_ceil(page_size))
}

/// Whether paging forward from 0-indexed `page` is allowed. With an
/// unknown page count, a full current page is taken as evidence more
/// may follow; a short page is the end.
fn can_advance_page(page: usize, total_pages: Option<usize>, is_full_page: bool) -> bool {
    match total_pages {
        Some(count) => page + 1 < count,
        None => is_full_page,
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
    ToggleSelectHovered,
    SelectAllVisible,
    InvertSelectionVisible,
    ClearSelection,
    ReadSelected,
    StartReplyToSelected {
        reply_all: bool,
    },
    StartForwardSelected,
    /// Transfer targets are mailbox ids, per [`Mailbox::id`]'s
    /// contract; display names are looked up only for status text.
    CopySelectedTo(String),
    MoveSelectedTo(String),
    FlagSelected {
        add: bool,
        action: FlagAction,
    },
    SendCompose,
    PreviewCompose,
    SaveComposeToDrafts,
    CancelCompose,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_total_yields_page_count() {
        assert_eq!(total_pages(Some(120), 50), Some(3));
        assert_eq!(total_pages(Some(50), 50), Some(1));
        assert_eq!(total_pages(Some(0), 50), Some(0));
    }

    #[test]
    fn unknown_total_yields_no_page_count() {
        assert_eq!(total_pages(None, 50), None);
    }

    #[test]
    fn known_total_bounds_advancing() {
        assert!(can_advance_page(1, Some(3), true));
        assert!(!can_advance_page(2, Some(3), true));
    }

    #[test]
    fn full_page_with_unknown_total_can_advance() {
        assert!(can_advance_page(0, None, true));
        assert!(!can_advance_page(0, None, false));
    }

    #[test]
    fn bulk_targets_fall_back_to_the_hovered_envelope() {
        let model = Model {
            envelopes: vec![Envelope::stub_with_id("1"), Envelope::stub_with_id("2")],
            envelope_index: 1,
            ..Model::default()
        };

        assert_eq!(model.bulk_target_ids(), vec!["2".to_string()]);
    }

    #[test]
    fn bulk_targets_use_the_selection_in_page_order() {
        let mut model = Model {
            envelopes: vec![
                Envelope::stub_with_id("1"),
                Envelope::stub_with_id("2"),
                Envelope::stub_with_id("3"),
            ],
            ..Model::default()
        };
        model.selected_envelope_ids.insert("3".into());
        model.selected_envelope_ids.insert("1".into());

        assert_eq!(
            model.bulk_target_ids(),
            vec!["1".to_string(), "3".to_string()]
        );
    }

    #[test]
    fn bulk_targets_are_empty_without_envelopes() {
        let model = Model::default();

        assert!(model.bulk_target_ids().is_empty());
    }
}
