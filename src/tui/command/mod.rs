//! App-wide command registry: the single source of action metadata —
//! name, aliases, key hint, context, availability and the dispatched
//! [`Message`]. Consumers (the Enter action menus, the command
//! palette, a future help overlay) only read it; execution always
//! flows through [`crate::tui::update`].

mod registry;

pub use registry::COMMANDS;

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::model::{Keybinds, Message, Model};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandContext {
    Global,
    Envelope,
    Composer,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyBinding {
    /// `None` binds in both flavors.
    pub flavor: Option<Keybinds>,
    pub code: KeyCode,
    pub modifiers: KeyModifiers,
}

impl KeyBinding {
    fn is_active(&self, flavor: Keybinds) -> bool {
        self.flavor.is_none_or(|f| f == flavor)
    }

    pub(crate) fn matches(&self, key: KeyEvent, flavor: Keybinds) -> bool {
        self.is_active(flavor) && self.code == key.code && self.accepts(key.modifiers)
    }

    /// Depending on the layout, a shifted symbol (e.g. `R`, `+`) may
    /// arrive with the SHIFT modifier still set.
    fn accepts(&self, actual: KeyModifiers) -> bool {
        if self.modifiers == KeyModifiers::NONE {
            matches!(actual, KeyModifiers::NONE | KeyModifiers::SHIFT)
        } else {
            self.modifiers == actual
        }
    }

    fn label(&self) -> String {
        let key = match self.code {
            KeyCode::Char(' ') => "Space".to_string(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::PageDown => "PgDn".to_string(),
            KeyCode::PageUp => "PgUp".to_string(),
            KeyCode::F(n) => format!("F{n}"),
            other => format!("{other:?}"),
        };
        let mut label = String::new();
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            label.push_str("Ctrl-");
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            label.push_str("Alt-");
        }
        label.push_str(&key);
        label
    }
}

/// A palette argument candidate: the label shown and completed, and
/// the message dispatched when it is confirmed.
pub struct Candidate {
    pub label: String,
    pub message: Message,
}

/// Fuzzy matching ranks candidates by their label.
impl AsRef<str> for Candidate {
    fn as_ref(&self) -> &str {
        &self.label
    }
}

/// Where an argument-taking command's candidates come from.
pub type CandidateSource = fn(&Model) -> Vec<Candidate>;

pub struct Command {
    /// Stable kebab-case handle: the typeable palette token, future
    /// keymap config and help.
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub bindings: &'static [KeyBinding],
    pub context: CommandContext,
    pub is_available: fn(&Model) -> bool,
    pub message: fn() -> Message,
    pub arg: Option<CandidateSource>,
}

impl Command {
    /// The typeable palette tokens.
    pub fn tokens(&self) -> impl Iterator<Item = &'static str> {
        std::iter::once(self.id).chain(self.aliases.iter().copied())
    }

    /// Display label of the first binding active in `flavor`.
    pub fn hint(&self, flavor: Keybinds) -> Option<String> {
        self.bindings
            .iter()
            .find(|b| b.is_active(flavor))
            .map(KeyBinding::label)
    }

    pub fn dispatch(&self) -> Message {
        (self.message)()
    }
}

pub fn for_context(context: CommandContext) -> impl Iterator<Item = &'static Command> {
    COMMANDS.iter().filter(move |c| c.context == context)
}

/// The available command bound to `key` under the model's flavor.
pub fn bound(model: &Model, key: KeyEvent) -> Option<&'static Command> {
    COMMANDS.iter().find(|c| {
        c.bindings.iter().any(|b| b.matches(key, model.keybinds)) && (c.is_available)(model)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{email::envelope::Envelope, tui::model::Panel};

    fn by_id(id: &str) -> &'static Command {
        COMMANDS.iter().find(|c| c.id == id).unwrap()
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
            assert_eq!(
                by_id("select-toggle").hint(flavor),
                Some("Space".to_string())
            );
            assert_eq!(by_id("select-clear").hint(flavor), None);
            assert_eq!(by_id("quit").hint(flavor), None);
            assert_eq!(by_id("send").hint(flavor), None);
        }
    }
}
