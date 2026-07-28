//! App-wide command registry: the single source of action metadata —
//! name, aliases, key hint, context, availability and the dispatched
//! [`Message`]. Consumers (the Enter action menus, the command
//! palette, a future help overlay) only read it; execution always
//! flows through [`crate::tui::update`].

mod registry;
#[cfg(test)]
mod tests;

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

pub struct Command {
    /// Stable kebab-case handle for future keymap config and help.
    pub id: &'static str,
    pub name: &'static str,
    pub aliases: &'static [&'static str],
    pub bindings: &'static [KeyBinding],
    pub context: CommandContext,
    pub is_available: fn(&Model) -> bool,
    pub message: fn() -> Message,
}

impl Command {
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
