use std::collections::HashSet;

use ratatui::crossterm::event::KeyEvent;

use super::*;
use crate::{
    config::DEFAULT_PALETTE_KEY,
    email::envelope::Envelope,
    tui::{
        model::{BottomPanel, Panel},
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

#[test]
fn envelope_menu_preserves_previous_dialog_rows() {
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
            "Remove flag"
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

#[test]
fn aliases_are_non_empty_lowercase_match_terms() {
    for command in COMMANDS {
        for alias in command.aliases {
            assert!(
                !alias.is_empty() && !alias.chars().any(|c| c.is_uppercase()),
                "alias {alias:?} of {} is not a lowercase match term",
                command.id
            );
        }
    }
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

fn bound_id(model: &Model, code: KeyCode, modifiers: KeyModifiers) -> Option<&'static str> {
    bound(model, KeyEvent::new(code, modifiers)).map(|c| c.id)
}

#[test]
fn paging_keys_are_flavor_scoped() {
    let mut model = Model {
        active_panel: Panel::Envelopes,
        ..Model::default()
    };

    assert_eq!(
        bound_id(&model, KeyCode::Char('d'), KeyModifiers::CONTROL),
        Some("next-page"),
        "Vim is the default flavor"
    );
    assert_eq!(
        bound_id(&model, KeyCode::Char('v'), KeyModifiers::CONTROL),
        None
    );

    model.keybinds = Keybinds::Emacs;
    assert_eq!(
        bound_id(&model, KeyCode::Char('d'), KeyModifiers::CONTROL),
        None
    );
    assert_eq!(
        bound_id(&model, KeyCode::Char('v'), KeyModifiers::CONTROL),
        Some("next-page")
    );
    assert_eq!(
        bound_id(&model, KeyCode::Char('v'), KeyModifiers::ALT),
        Some("previous-page")
    );

    assert_eq!(
        bound_id(&model, KeyCode::PageDown, KeyModifiers::NONE),
        Some("next-page"),
        "PgDn stays flavor-neutral"
    );
}

#[test]
fn bound_requires_availability() {
    let mut model = Model::default();
    assert_eq!(
        bound_id(&model, KeyCode::Char('r'), KeyModifiers::NONE),
        None,
        "no envelope selected"
    );

    model.envelopes.push(Envelope::stub());
    assert_eq!(
        bound_id(&model, KeyCode::Char('r'), KeyModifiers::NONE),
        Some("reply"),
        "availability-only gating: fires even with the Mailboxes panel focused"
    );
}

#[test]
fn bound_accepts_shifted_symbols() {
    let mut model = Model::default();
    model.envelopes.push(Envelope::stub());

    assert_eq!(
        bound_id(&model, KeyCode::Char('R'), KeyModifiers::SHIFT),
        Some("reply-all")
    );
    assert_eq!(
        bound_id(&model, KeyCode::Char('+'), KeyModifiers::SHIFT),
        Some("add-flag")
    );
}

#[test]
fn hints_derive_from_bindings() {
    for flavor in [Keybinds::Vim, Keybinds::Emacs] {
        assert_eq!(
            by_id("new-message").hint(flavor),
            Some("Ctrl-c".to_string())
        );
        assert_eq!(by_id("next-page").hint(flavor), Some("PgDn".to_string()));
        assert_eq!(
            by_id("reload-mailboxes").hint(flavor),
            Some("F5".to_string())
        );
        assert_eq!(by_id("reply").hint(flavor), Some("r".to_string()));
        assert_eq!(by_id("quit").hint(flavor), None);
        assert_eq!(by_id("send").hint(flavor), None);
    }
}

#[test]
fn envelope_commands_require_a_selected_envelope() {
    let mut model = Model::default();
    assert!(!available(&model, CommandContext::Envelope));

    model.envelopes.push(Envelope::stub());
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
