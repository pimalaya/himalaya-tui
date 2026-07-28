//! Command palette: a fuzzy-searchable overlay over the command
//! registry. Owns its own state and transitions; executing a command
//! means returning its [`Message`] for [`crate::tui::update`] to fold
//! through the normal chain — the palette itself never performs I/O.

#[cfg(test)]
mod tests;
pub mod view;

use std::cell::RefCell;

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::Input;

use crate::tui::{
    command::{COMMANDS, Command},
    model::{Message, Model},
    update::input_request,
};

thread_local! {
    // A nucleo Matcher eagerly allocates a ~135KB scoring slab, so it
    // is built once and reused rather than per rank() call.
    static MATCHER: RefCell<Matcher> = RefCell::new(Matcher::new(Config::DEFAULT));
}

/// Number of result rows visible in the palette overlay.
pub const PALETTE_VISIBLE: usize = 10;

pub struct PaletteState {
    pub input: Input,
    pub selected: usize,
}

#[derive(Debug, Clone)]
pub enum PaletteMessage {
    Open,
    Close,
    Next,
    Previous,
    Confirm,
    Complete,
    Input(KeyEvent),
}

/// A registry command paired with its availability in the current
/// model state; unavailable entries render greyed and are inert.
pub struct PaletteEntry {
    pub command: &'static Command,
    pub is_available: bool,
}

pub fn translate_key(key: KeyEvent) -> PaletteMessage {
    match (key.code, key.modifiers) {
        (KeyCode::Esc, _) => PaletteMessage::Close,
        (KeyCode::Enter, _) => PaletteMessage::Confirm,
        (KeyCode::Tab, _) => PaletteMessage::Complete,
        (KeyCode::Down, _) => PaletteMessage::Next,
        (KeyCode::Up, _) => PaletteMessage::Previous,
        (KeyCode::Char('n'), KeyModifiers::CONTROL) => PaletteMessage::Next,
        (KeyCode::Char('p'), KeyModifiers::CONTROL) => PaletteMessage::Previous,
        _ => PaletteMessage::Input(key),
    }
}

pub fn update(model: &mut Model, msg: PaletteMessage) -> Option<Message> {
    match msg {
        PaletteMessage::Open => {
            model.palette = Some(PaletteState {
                input: Input::default(),
                selected: 0,
            });
            None
        }
        PaletteMessage::Close => {
            model.palette = None;
            None
        }
        PaletteMessage::Next => {
            move_selection(model, true);
            None
        }
        PaletteMessage::Previous => {
            move_selection(model, false);
            None
        }
        PaletteMessage::Confirm => confirm(model),
        PaletteMessage::Complete => {
            complete_selected(model);
            None
        }
        PaletteMessage::Input(key) => {
            edit_filter(model, key);
            None
        }
    }
}

/// Commands matching the current filter, ranked available-first then
/// by fuzzy score; empty filter yields the whole registry in table
/// order (still available-first).
pub fn entries(model: &Model) -> Vec<PaletteEntry> {
    let Some(palette) = &model.palette else {
        return Vec::new();
    };
    let candidates = COMMANDS
        .iter()
        .map(|command| PaletteEntry {
            command,
            is_available: (command.is_available)(model),
        })
        .collect();
    rank(palette.input.value(), candidates)
}

fn rank(filter: &str, mut candidates: Vec<PaletteEntry>) -> Vec<PaletteEntry> {
    if filter.is_empty() {
        // Stable, so registry order is preserved within each group.
        candidates.sort_by_key(|e| !e.is_available);
        return candidates;
    }

    let pattern = Pattern::parse(filter, CaseMatching::Ignore, Normalization::Smart);
    MATCHER.with_borrow_mut(|matcher| {
        let mut scored: Vec<(u32, PaletteEntry)> = candidates
            .drain(..)
            .filter_map(|entry| {
                best_score(matcher, &pattern, entry.command).map(|score| (score, entry))
            })
            .collect();
        scored.sort_by_key(|(score, entry)| (!entry.is_available, std::cmp::Reverse(*score)));
        scored.into_iter().map(|(_, entry)| entry).collect()
    })
}

fn best_score(matcher: &mut Matcher, pattern: &Pattern, command: &Command) -> Option<u32> {
    let mut buf = Vec::new();
    std::iter::once(&command.name)
        .chain(command.aliases)
        .filter_map(|text| pattern.score(Utf32Str::new(text, &mut buf), matcher))
        .max()
}

fn confirm(model: &mut Model) -> Option<Message> {
    let ranked_entries = entries(model);
    let selected_entry = ranked_entries.get(model.palette.as_ref()?.selected)?;
    if !selected_entry.is_available {
        return None;
    }
    let confirmed = selected_entry.command.dispatch();
    model.palette = None;
    Some(confirmed)
}

/// Vim-wildmenu style Tab: put the highlighted command's name into
/// the filter and keep it highlighted under the new ranking.
fn complete_selected(model: &mut Model) {
    let selected = match &model.palette {
        Some(palette) => palette.selected,
        None => return,
    };
    let Some(command) = entries(model).get(selected).map(|e| e.command) else {
        return;
    };

    if let Some(palette) = &mut model.palette {
        palette.input = Input::new(command.name.to_string());
    }
    let position = entries(model)
        .iter()
        .position(|e| e.command.id == command.id);
    if let (Some(position), Some(palette)) = (position, &mut model.palette) {
        palette.selected = position;
    }
}

fn move_selection(model: &mut Model, forward: bool) {
    let count = entries(model).len();
    let Some(palette) = &mut model.palette else {
        return;
    };
    if count == 0 {
        return;
    }
    palette.selected = if forward {
        (palette.selected + 1) % count
    } else {
        palette.selected.checked_sub(1).unwrap_or(count - 1)
    };
}

fn edit_filter(model: &mut Model, key: KeyEvent) {
    let Some(palette) = &mut model.palette else {
        return;
    };
    let Some(req) = input_request(key) else {
        return;
    };
    if palette.input.handle(req).is_some_and(|change| change.value) {
        palette.selected = 0;
    }
}
