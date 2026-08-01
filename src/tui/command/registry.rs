use ratatui::crossterm::event::{KeyCode, KeyModifiers};

use super::{Candidate, Command, CommandContext, KeyBinding};
use crate::tui::model::{BottomPanel, Dialog, FlagAction, Keybinds, Message, Model, Panel};

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

fn transfer_candidates(model: &Model, message: fn(String) -> Message) -> Vec<Candidate> {
    model
        .mailboxes
        .iter()
        .map(|m| Candidate {
            label: m.name.clone(),
            message: message(m.id.clone()),
        })
        .collect()
}

fn copy_candidates(model: &Model) -> Vec<Candidate> {
    transfer_candidates(model, Message::CopySelectedTo)
}

fn move_candidates(model: &Model) -> Vec<Candidate> {
    transfer_candidates(model, Message::MoveSelectedTo)
}

fn flag_candidates(add: bool) -> Vec<Candidate> {
    FlagAction::ALL
        .iter()
        .map(|action| Candidate {
            label: action.label().to_string(),
            message: Message::FlagSelected {
                add,
                action: *action,
            },
        })
        .collect()
}

fn add_flag_candidates(_: &Model) -> Vec<Candidate> {
    flag_candidates(true)
}

fn remove_flag_candidates(_: &Model) -> Vec<Candidate> {
    flag_candidates(false)
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
        arg: None,
    },
    Command {
        id: "reply",
        name: "Reply",
        aliases: &[],
        bindings: &[plain('r')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::StartReplyToSelected { reply_all: false },
        arg: None,
    },
    Command {
        id: "reply-all",
        name: "Reply All",
        aliases: &[],
        bindings: &[plain('R')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::StartReplyToSelected { reply_all: true },
        arg: None,
    },
    Command {
        id: "forward",
        name: "Forward",
        aliases: &[],
        bindings: &[plain('f')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::StartForwardSelected,
        arg: None,
    },
    Command {
        id: "copy",
        name: "Copy",
        aliases: &[],
        bindings: &[plain('c')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::CopyTo),
        arg: Some(copy_candidates),
    },
    Command {
        id: "move",
        name: "Move",
        aliases: &[],
        bindings: &[plain('m')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::MoveTo),
        arg: Some(move_candidates),
    },
    Command {
        id: "add-flag",
        name: "Add flag",
        aliases: &[],
        bindings: &[plain('+')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::FlagAdd),
        arg: Some(add_flag_candidates),
    },
    Command {
        id: "remove-flag",
        name: "Remove flag",
        aliases: &[],
        bindings: &[plain('-')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::OpenDialog(Dialog::FlagRemove),
        arg: Some(remove_flag_candidates),
    },
    Command {
        id: "select-toggle",
        name: "Toggle selection",
        aliases: &["mark", "select"],
        bindings: &[plain(' ')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::ToggleSelectHovered,
        arg: None,
    },
    Command {
        id: "select-all",
        name: "Select all",
        aliases: &["mark-all"],
        bindings: &[chord(None, KeyModifiers::CONTROL, 'a')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::SelectAllVisible,
        arg: None,
    },
    Command {
        id: "select-invert",
        name: "Invert selection",
        aliases: &["toggle-all"],
        bindings: &[chord(None, KeyModifiers::CONTROL, 'r')],
        context: CommandContext::Envelope,
        is_available: has_selected_envelope,
        message: || Message::InvertSelectionVisible,
        arg: None,
    },
    Command {
        id: "select-clear",
        name: "Clear selection",
        aliases: &["unmark", "deselect"],
        bindings: &[],
        context: CommandContext::Envelope,
        is_available: Model::has_selection,
        message: || Message::ClearSelection,
        arg: None,
    },
    Command {
        id: "send",
        name: "Send",
        aliases: &[],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::SendCompose,
        arg: None,
    },
    Command {
        id: "preview",
        name: "Preview",
        aliases: &[],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::PreviewCompose,
        arg: None,
    },
    Command {
        id: "save-to-drafts",
        name: "Save to Drafts",
        aliases: &["draft"],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::SaveComposeToDrafts,
        arg: None,
    },
    Command {
        id: "cancel",
        name: "Cancel",
        aliases: &["discard"],
        bindings: &[],
        context: CommandContext::Composer,
        is_available: composer_active,
        message: || Message::CancelCompose,
        arg: None,
    },
    Command {
        id: "new-message",
        name: "New message",
        aliases: &["compose"],
        bindings: &[chord(None, KeyModifiers::CONTROL, 'c')],
        context: CommandContext::Global,
        is_available: always,
        message: || Message::StartCompose,
        arg: None,
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
        arg: None,
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
        arg: None,
    },
    Command {
        id: "reload-mailboxes",
        name: "Reload mailboxes",
        aliases: &["refresh"],
        bindings: &[key(KeyCode::F(5))],
        context: CommandContext::Global,
        is_available: always,
        message: || Message::LoadMailboxes,
        arg: None,
    },
    Command {
        id: "quit",
        name: "Quit",
        aliases: &["exit"],
        bindings: &[],
        context: CommandContext::Global,
        is_available: always,
        message: || Message::Quit,
        arg: None,
    },
];

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::{
        config::DEFAULT_PALETTE_KEY,
        email::envelope::Envelope,
        tui::{
            command::for_context,
            update::{CTRL_ALIASES, PLAIN_ALIASES},
        },
    };

    fn by_id(id: &str) -> &'static Command {
        COMMANDS.iter().find(|c| c.id == id).unwrap()
    }

    fn labels(context: CommandContext) -> Vec<&'static str> {
        for_context(context).map(|c| c.name).collect()
    }

    fn available(model: &Model, context: CommandContext) -> bool {
        for_context(context).all(|c| (c.is_available)(model))
    }

    #[test]
    fn ids_are_unique() {
        let mut seen = HashSet::new();
        for command in COMMANDS {
            assert!(
                seen.insert(command.id),
                "duplicate command id: {}",
                command.id
            );
        }
    }

    #[test]
    fn ids_are_kebab_case() {
        for command in COMMANDS {
            assert!(
                !command.id.is_empty()
                    && command
                        .id
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '-'),
                "command id is not kebab-case: {}",
                command.id
            );
        }
    }

    // The argument parser splits at the first whitespace; ids are already
    // whitespace-free per the kebab-case test.
    #[test]
    fn aliases_are_single_lowercase_tokens() {
        for command in COMMANDS {
            for alias in command.aliases {
                assert!(
                    !alias.is_empty()
                        && !alias.chars().any(|c| c.is_uppercase())
                        && !alias.contains(char::is_whitespace),
                    "alias {alias:?} of {} is not a single lowercase token",
                    command.id
                );
            }
        }
    }

    #[test]
    fn envelope_menu_lists_registry_rows_in_order() {
        assert_eq!(
            labels(CommandContext::Envelope),
            [
                "Read",
                "Reply",
                "Reply All",
                "Forward",
                "Copy",
                "Move",
                "Add flag",
                "Remove flag",
                "Toggle selection",
                "Select all",
                "Invert selection",
                "Clear selection"
            ]
        );
    }

    #[test]
    fn composer_menu_preserves_previous_dialog_rows() {
        assert_eq!(
            labels(CommandContext::Composer),
            ["Send", "Preview", "Save to Drafts", "Cancel"]
        );
    }

    fn active_bindings(flavor: Keybinds) -> Vec<(&'static str, KeyCode, KeyModifiers)> {
        COMMANDS
            .iter()
            .flat_map(|c| {
                c.bindings
                    .iter()
                    .filter(move |b| b.is_active(flavor))
                    .map(move |b| (c.id, b.code, b.modifiers))
            })
            .collect()
    }

    #[test]
    fn bindings_are_unique_per_flavor() {
        for flavor in [Keybinds::Vim, Keybinds::Emacs] {
            let mut seen = HashSet::new();
            for (id, code, modifiers) in active_bindings(flavor) {
                assert!(
                    seen.insert((code, modifiers)),
                    "duplicate {flavor:?} binding {code:?}+{modifiers:?} (last on {id})"
                );
            }
        }
    }

    #[test]
    fn bindings_avoid_reserved_keys() {
        let structural = [
            KeyCode::Esc,
            KeyCode::Tab,
            KeyCode::Down,
            KeyCode::Up,
            KeyCode::Enter,
        ];
        let aliased_plain: Vec<char> = PLAIN_ALIASES
            .iter()
            .map(|(c, _)| *c)
            .chain([DEFAULT_PALETTE_KEY])
            .collect();
        let aliased_ctrl: Vec<char> = CTRL_ALIASES.iter().map(|(c, _)| *c).collect();

        for flavor in [Keybinds::Vim, Keybinds::Emacs] {
            for (id, code, modifiers) in active_bindings(flavor) {
                assert!(
                    !structural.contains(&code),
                    "{id} binds structural key {code:?}"
                );
                if let KeyCode::Char(c) = code {
                    assert!(
                        !(modifiers == KeyModifiers::NONE && aliased_plain.contains(&c)),
                        "{id} binds reserved plain key {c:?}"
                    );
                    assert!(
                        !(modifiers == KeyModifiers::CONTROL && aliased_ctrl.contains(&c)),
                        "{id} binds reserved Ctrl key {c:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn envelope_commands_require_a_selected_envelope() {
        let mut model = Model::default();
        assert!(!available(&model, CommandContext::Envelope));

        model.envelopes.push(Envelope::stub());
        let unavailable: Vec<&str> = for_context(CommandContext::Envelope)
            .filter(|c| !(c.is_available)(&model))
            .map(|c| c.id)
            .collect();
        assert_eq!(
            unavailable,
            ["select-clear"],
            "clearing is the one envelope command that needs an existing selection"
        );

        model.selected_envelope_ids.insert("1".into());
        assert!(available(&model, CommandContext::Envelope));
    }

    #[test]
    fn composer_commands_require_an_active_composer() {
        let mut model = Model::default();
        assert!(!available(&model, CommandContext::Composer));

        model.bottom_panel = BottomPanel::Compose;
        assert!(available(&model, CommandContext::Composer));

        model.bottom_panel = BottomPanel::MessagePreview;
        assert!(
            available(&model, CommandContext::Composer),
            "previewing keeps the compose buffer alive, so composer commands stay available"
        );
    }

    #[test]
    fn paging_requires_envelope_panel_focus() {
        let mut model = Model::default();
        let paging_available = |m: &Model| {
            [by_id("next-page"), by_id("previous-page")]
                .iter()
                .all(|c| (c.is_available)(m))
        };

        assert!(!paging_available(&model));
        model.active_panel = Panel::Envelopes;
        assert!(paging_available(&model));
    }

    #[test]
    fn global_actions_are_always_available() {
        let model = Model::default();
        for id in ["new-message", "reload-mailboxes", "quit"] {
            assert!(
                (by_id(id).is_available)(&model),
                "{id} should never grey out"
            );
        }
    }
}
