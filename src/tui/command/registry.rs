use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use super::{Command, CommandContext, KeyBinding};
use crate::tui::model::{BottomPanel, Dialog, Keybinds, Message, Model, Panel};

const fn key(code: KeyCode) -> KeyBinding {
    KeyBinding {
        flavor: None,
        code,
        modifiers: KeyModifiers::NONE,
    }
}

const fn plain(c: char) -> KeyBinding {
    key(KeyCode::Char(c))
}

const fn chord(flavor: Option<Keybinds>, modifiers: KeyModifiers, c: char) -> KeyBinding {
    KeyBinding {
        flavor,
        code: KeyCode::Char(c),
        modifiers,
    }
}

fn always(_: &Model) -> bool {
    true
}

fn has_selected_envelope(model: &Model) -> bool {
    model.selected_envelope().is_some()
}

fn composer_active(model: &Model) -> bool {
    matches!(
        model.bottom_panel,
        BottomPanel::Compose | BottomPanel::MessagePreview
    )
}

fn envelopes_focused(model: &Model) -> bool {
    model.active_panel == Panel::Envelopes
}

pub const COMMANDS: &[Command] = &[
    Command {
        id: "read",
        name: "Read",
        aliases: &["open", "view"],
        bindings: &[plain('o')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::ReadSelected,
    },
    Command {
        id: "reply",
        name: "Reply",
        aliases: &[],
        bindings: &[plain('r')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::StartReplyToSelected { reply_all: false },
    },
    Command {
        id: "reply-all",
        name: "Reply All",
        aliases: &[],
        bindings: &[plain('R')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::StartReplyToSelected { reply_all: true },
    },
    Command {
        id: "forward",
        name: "Forward",
        aliases: &[],
        bindings: &[plain('f')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::StartForwardSelected,
    },
    Command {
        id: "copy",
        name: "Copy",
        aliases: &["copy to"],
        bindings: &[plain('c')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::CopyTo),
    },
    Command {
        id: "move",
        name: "Move",
        aliases: &["move to"],
        bindings: &[plain('m')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::MoveTo),
    },
    Command {
        id: "add-flag",
        name: "Add flag",
        aliases: &[],
        bindings: &[plain('+')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::FlagAdd),
    },
    Command {
        id: "remove-flag",
        name: "Remove flag",
        aliases: &[],
        bindings: &[plain('-')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::FlagRemove),
    },
    Command {
        id: "send",
        name: "Send",
        aliases: &[],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::SendCompose,
    },
    Command {
        id: "preview",
        name: "Preview",
        aliases: &[],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::PreviewCompose,
    },
    Command {
        id: "save-to-drafts",
        name: "Save to Drafts",
        aliases: &["draft"],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::SaveComposeToDrafts,
    },
    Command {
        id: "cancel",
        name: "Cancel",
        aliases: &["discard"],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::CancelCompose,
    },
    Command {
        id: "new-message",
        name: "New message",
        aliases: &["compose"],
        bindings: &[chord(None, KeyModifiers::CONTROL, 'c')],
        context: CommandContext::Global,
        is_available: always,
        message: || Message::StartCompose,
    },
    Command {
        id: "next-page",
        name: "Next page",
        aliases: &[],
        bindings: &[
            key(KeyCode::PageDown),
            chord(Some(Keybinds::Vim), KeyModifiers::CONTROL, 'd'),
            chord(Some(Keybinds::Emacs), KeyModifiers::CONTROL, 'v'),
        ],
        context: CommandContext::Global,
        is_available: envelopes_focused,
        message: || Message::PageDown,
    },
    Command {
        id: "previous-page",
        name: "Previous page",
        aliases: &[],
        bindings: &[
            key(KeyCode::PageUp),
            chord(Some(Keybinds::Vim), KeyModifiers::CONTROL, 'u'),
            chord(Some(Keybinds::Emacs), KeyModifiers::ALT, 'v'),
        ],
        context: CommandContext::Global,
        is_available: envelopes_focused,
        message: || Message::PageUp,
    },
    Command {
        id: "reload-mailboxes",
        name: "Reload mailboxes",
        aliases: &["refresh"],
        bindings: &[key(KeyCode::F(5))],
        context: CommandContext::Global,
        is_available: always,
        message: || Message::LoadMailboxes,
    },
    Command {
        id: "quit",
        name: "Quit",
        aliases: &["exit"],
        bindings: &[],
        context: CommandContext::Global,
        is_available: always,
        message: || Message::Quit,
    },
];
