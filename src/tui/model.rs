//! # Model
//!
//! Model layer of the Elm Architecture: every piece of TUI state, plus
//! the [`Message`] enum naming each transition. Mutation lives in
//! [`crate::tui::update`], rendering in [`crate::tui::view`].

use std::time::{Duration, Instant};

use clap::ValueEnum;
use edtui::{EditorEventHandler, EditorState};
use pimalaya_config::command::CommandConfig;
use ratatui::crossterm::event::KeyEvent;
use serde::{Deserialize, Serialize};
use tui_input::Input;

use crate::{
    client::MailClient,
    contact::{Contact, ContactCompleteKey, ContactLookup, ContactTarget},
    email::{
        envelope::Envelope,
        flag::{Flag, IanaFlag},
        mailbox::Mailbox,
    },
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

/// Contact rows visible at once in the completion list.
pub const CONTACT_LIST_VISIBLE: usize = 8;

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
    /// Messages the selected mailbox holds, as himalaya's listing
    /// reported it, `None` when its backend does not say.
    pub envelope_total: Option<u64>,
    /// Whether the last listing named a next page.
    pub envelope_has_next: bool,
    /// The first page known to come back empty, which a full last page
    /// leads to when no total says where the mailbox ends.
    pub envelope_end: Option<usize>,
    /// Ids of the envelopes marked for an action, in marking order. They
    /// belong to the selected mailbox and survive paging.
    pub marked: Vec<String>,
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
    /// Blocking action run on the next frame, once the status announcing
    /// it is drawn.
    pub deferred: Option<Message>,
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
    /// Command answering the contacts matching a recipient fragment.
    pub contact_command: Option<CommandConfig>,
    /// Key completing the recipient under the cursor.
    pub contact_complete_key: ContactCompleteKey,
    /// Contact command running, if any.
    pub contact_lookup: Option<ContactLookup>,
    /// Completion list open over the composer, if any.
    pub contact_completion: Option<ContactCompletion>,
    /// Resolved colors every render function reads.
    pub theme: Theme,
    /// The himalaya session every mail action goes through.
    pub client: MailClient,
    /// Timestamp of the last round-trip, or of the last keepalive.
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

    /// The ids an action reaches: the marked envelopes when any are,
    /// else the one the cursor is on.
    pub fn action_ids(&self) -> Vec<String> {
        if !self.marked.is_empty() {
            return self.marked.clone();
        }

        self.selected_envelope()
            .map(|envelope| vec![envelope.id.clone()])
            .unwrap_or_default()
    }

    /// Whether `id` is marked.
    pub fn is_marked(&self, id: &str) -> bool {
        self.marked.iter().any(|marked| marked == id)
    }

    /// Role of the mailbox the envelopes were loaded from, if any.
    pub fn selected_mailbox_role(&self) -> Option<&str> {
        let id = self.selected_mailbox.as_deref()?;
        self.mailboxes
            .iter()
            .find(|m| m.id == id)
            .and_then(|m| m.role.as_deref())
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
            Some(Dialog::ConfirmDelete) => 2,
            None => 0,
        }
    }

    /// Number of pages the selected mailbox spans, at least one, `None`
    /// when its backend reports no total.
    pub fn total_pages(&self) -> Option<usize> {
        total_pages(self.envelope_total, self.envelope_page_size)
    }

    /// Whether a page follows the loaded one: by the total when known,
    /// else by the listing's next page, short of a page known empty.
    pub fn has_next_page(&self) -> bool {
        has_next_page(
            self.total_pages(),
            self.envelope_page,
            self.envelope_has_next,
            self.envelope_end,
        )
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
    /// Confirmation of a delete from the trash, which is for good.
    ConfirmDelete,
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
    /// Delete the message, trash first.
    Delete,
}

impl EnvelopeAction {
    /// Every envelope action, in dialog order.
    pub const ALL: [EnvelopeAction; 9] = [
        EnvelopeAction::Read,
        EnvelopeAction::Reply,
        EnvelopeAction::ReplyAll,
        EnvelopeAction::Forward,
        EnvelopeAction::Copy,
        EnvelopeAction::Move,
        EnvelopeAction::AddFlag,
        EnvelopeAction::RemoveFlag,
        EnvelopeAction::Delete,
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
            EnvelopeAction::Delete => "Delete",
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

    /// The flag's name as himalaya's `flag add` takes it.
    pub fn name(&self) -> &'static str {
        match self {
            FlagAction::Seen => "seen",
            FlagAction::Flagged => "flagged",
            FlagAction::Answered => "answered",
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

/// Contacts a lookup answered, listed at the cursor for a pick.
#[derive(Debug, Clone)]
pub struct ContactCompletion {
    /// Where in the buffer the pick goes.
    pub target: ContactTarget,
    /// Contacts the command answered, two or more.
    pub contacts: Vec<Contact>,
    /// Index of the highlighted contact.
    pub index: usize,
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
    /// Toggle the mark of the focused envelope, then move down.
    ToggleMark,
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
    /// Run the contact command on the fragment under the cursor.
    CompleteContact(ContactTarget),
    /// Collect the contact command's answer, if it has one.
    PollContactLookup,
    /// Highlight the next contact of the completion list.
    ContactCompletionNext,
    /// Highlight the previous contact of the completion list.
    ContactCompletionPrevious,
    /// Insert the highlighted contact.
    ContactCompletionAccept,
    /// Close the completion list, handing the key back for translation.
    ContactCompletionDismiss(Option<KeyEvent>),
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
    /// Keep the connection alive after an idle period.
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
    /// Delete the marked or the selected messages, asking first in the
    /// trash.
    DeleteSelected,
    /// Send the compose buffer.
    SendCompose,
    /// Compile the compose buffer and show the result.
    PreviewCompose,
    /// Save the compose buffer to the drafts mailbox.
    SaveComposeToDrafts,
    /// Discard the compose buffer.
    CancelCompose,
    /// Run a blocking action announced on the previous frame.
    Run(Blocking),
}

/// An action blocking the loop, announced one frame before it runs.
///
/// Carries only what the model does not hold: the selection and the
/// dialog it reads stay put between the two frames, no key being
/// handled in between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocking {
    /// Fetch the current envelope page and land the cursor.
    LoadEnvelopes(EnvelopeLanding),
    /// Fetch the selected message and show it.
    Read,
    /// Fetch the selected message and reply to it.
    Reply { reply_all: bool },
    /// Fetch the selected message and forward it.
    Forward,
    /// Copy the selected message to the picked mailbox.
    Copy,
    /// Move the selected message to the picked mailbox.
    Move,
    /// Add or remove the picked flag on the selected message.
    Flag { add: bool },
    /// Delete the marked or the selected messages.
    Delete,
    /// Compile and send the compose buffer.
    Send,
    /// Save the compose buffer to the drafts mailbox.
    SaveDraft,
}

/// Number of pages `total` messages span at `page_size` a page, at least
/// one, `None` without a total.
fn total_pages(total: Option<u64>, page_size: usize) -> Option<usize> {
    let total = usize::try_from(total?).ok()?;

    if page_size == 0 || total == 0 {
        Some(1)
    } else {
        Some(total.div_ceil(page_size))
    }
}

/// Whether a page follows the zero-based `page`: by the page count when
/// known, else by the listing's next page, short of the page `end` known
/// to come back empty.
fn has_next_page(pages: Option<usize>, page: usize, has_next: bool, end: Option<usize>) -> bool {
    match pages {
        Some(pages) => page + 1 < pages,
        None => has_next && end != Some(page + 1),
    }
}

#[cfg(test)]
impl Model {
    /// A model over `client`, nothing loaded, pages of `page_size`.
    pub fn for_tests(client: MailClient, page_size: usize) -> Self {
        Self {
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
            envelope_page_size: page_size,
            envelope_capacity: 0,
            envelope_total: None,
            envelope_has_next: false,
            envelope_end: None,
            marked: Vec::new(),
            selected_mailbox: None,
            account_name: String::from("test"),
            from: None,
            from_name: None,
            signature: String::new(),
            status_message: None,
            deferred: None,
            bottom_panel: BottomPanel::None,
            message_content: None,
            message_scroll: 0,
            editor_state: EditorState::new(edtui::Lines::from("")),
            editor_handler: Keybinds::Vim.editor_handler(),
            dialog: None,
            dialog_index: 0,
            keybinds: None,
            contact_command: None,
            contact_complete_key: ContactCompleteKey::default(),
            contact_lookup: None,
            contact_completion: None,
            theme: Theme::default(),
            client,
            last_activity: Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{has_next_page, total_pages};

    #[test]
    fn a_total_decides_the_page_count() {
        assert_eq!(total_pages(Some(120), 25), Some(5));
        assert_eq!(total_pages(Some(25), 25), Some(1));
        assert_eq!(total_pages(Some(0), 25), Some(1));
        assert_eq!(total_pages(None, 25), None);

        assert!(has_next_page(Some(5), 3, false, None));
        assert!(!has_next_page(Some(5), 4, true, None));
    }

    /// Without a total, a full last page names a next one that comes back
    /// empty: once seen empty, it is no longer offered.
    #[test]
    fn without_a_total_the_page_known_empty_ends_the_mailbox() {
        assert!(has_next_page(None, 1, true, None));
        assert!(!has_next_page(None, 1, true, Some(2)));
        assert!(!has_next_page(None, 1, false, None));
        assert!(has_next_page(None, 0, true, Some(2)));
    }
}
