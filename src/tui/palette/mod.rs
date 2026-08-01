//! Command palette: a fuzzy-searchable overlay over the command
//! registry. Owns its own state and transitions; executing a command
//! means returning its [`Message`] for [`crate::tui::update`] to fold
//! through the normal chain — the palette itself never performs I/O.

pub mod args;
pub mod view;

use std::cell::RefCell;

use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_input::Input;

use crate::tui::{
    command::{COMMANDS, Candidate, Command},
    model::{Keybinds, Message, Model},
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

/// One visible palette row, uniform across command and argument mode:
/// what to show, what Tab writes into the filter, and what Enter
/// dispatches (`None` renders greyed and is inert).
pub struct PaletteRow {
    pub label: String,
    pub hint: String,
    pub is_available: bool,
    pub completion: String,
    pub message: Option<Message>,
}

/// The rows currently listed; the only place the two palette modes
/// branch.
pub fn rows(model: &Model) -> Vec<PaletteRow> {
    match args::argument_query(model) {
        Some(query) => args::candidates(model, &query)
            .into_iter()
            .map(|candidate| argument_row(&query, candidate))
            .collect(),
        None => entries(model)
            .into_iter()
            .map(|entry| command_row(entry, model.keybinds))
            .collect(),
    }
}

fn command_row(entry: PaletteEntry, flavor: Keybinds) -> PaletteRow {
    let command = entry.command;
    let enters_argument_mode = entry.is_available && command.arg.is_some();
    PaletteRow {
        label: command.name.to_string(),
        hint: command.hint(flavor).unwrap_or_default(),
        is_available: entry.is_available,
        completion: if enters_argument_mode {
            format!("{} ", command.id)
        } else {
            command.id.to_string()
        },
        message: entry.is_available.then(|| command.dispatch()),
    }
}

fn argument_row(query: &args::ArgumentQuery, candidate: Candidate) -> PaletteRow {
    PaletteRow {
        completion: format!("{} {}", query.head, candidate.label),
        label: candidate.label,
        hint: String::new(),
        is_available: true,
        message: Some(candidate.message),
    }
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
    command
        .tokens()
        .chain([command.name])
        .filter_map(|text| pattern.score(Utf32Str::new(text, &mut buf), matcher))
        .max()
}

fn confirm(model: &mut Model) -> Option<Message> {
    let selected = model.palette.as_ref()?.selected;
    let confirmed = rows(model).into_iter().nth(selected)?.message?;
    model.palette = None;
    Some(confirmed)
}

/// Vim-wildmenu style Tab: write the highlighted row's completion
/// into the filter and keep that row highlighted under the new
/// ranking (completing into argument mode lands on the first
/// candidate instead).
fn complete_selected(model: &mut Model) {
    let Some(selected) = model.palette.as_ref().map(|p| p.selected) else {
        return;
    };
    let Some(completion) = rows(model).into_iter().nth(selected).map(|r| r.completion) else {
        return;
    };

    if let Some(palette) = &mut model.palette {
        palette.input = Input::new(completion.clone());
        palette.selected = 0;
    }
    let position = rows(model).iter().position(|r| r.completion == completion);
    if let (Some(position), Some(palette)) = (position, &mut model.palette) {
        palette.selected = position;
    }
}

fn move_selection(model: &mut Model, forward: bool) {
    let count = rows(model).len();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::model::BottomPanel;

    fn open_palette(model: &mut Model) {
        update(model, PaletteMessage::Open);
    }

    fn type_filter(model: &mut Model, text: &str) {
        for c in text.chars() {
            update(
                model,
                PaletteMessage::Input(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)),
            );
        }
    }

    fn entry_ids(model: &Model) -> Vec<&'static str> {
        entries(model).iter().map(|e| e.command.id).collect()
    }

    #[test]
    fn open_resets_input_and_selection() {
        let mut model = Model::default();
        open_palette(&mut model);
        type_filter(&mut model, "qu");
        update(&mut model, PaletteMessage::Next);

        open_palette(&mut model);
        let palette = model.palette.as_ref().unwrap();
        assert_eq!(palette.input.value(), "");
        assert_eq!(palette.selected, 0);
    }

    #[test]
    fn close_discards_palette_state() {
        let mut model = Model::default();
        open_palette(&mut model);
        update(&mut model, PaletteMessage::Close);
        assert!(model.palette.is_none());
    }

    #[test]
    fn empty_filter_lists_whole_registry_available_first() {
        let mut model = Model::default();
        open_palette(&mut model);

        let listed = entries(&model);
        assert_eq!(listed.len(), COMMANDS.len());
        let first_unavailable = listed
            .iter()
            .position(|e| !e.is_available)
            .unwrap_or(listed.len());
        assert!(
            listed[first_unavailable..].iter().all(|e| !e.is_available),
            "unavailable entries must all sort after available ones"
        );
    }

    #[test]
    fn filter_ranks_fuzzy_matches() {
        let mut model = Model::default();
        open_palette(&mut model);
        type_filter(&mut model, "qit");

        let ids = entry_ids(&model);
        assert!(
            ids.contains(&"quit"),
            "subsequence `qit` should fuzzy-match Quit, got {ids:?}"
        );
    }

    #[test]
    fn filter_matches_aliases() {
        let mut model = Model::default();
        open_palette(&mut model);
        type_filter(&mut model, "refresh");

        assert_eq!(entry_ids(&model), ["reload-mailboxes"]);
    }

    #[test]
    fn filter_change_resets_selection() {
        let mut model = Model::default();
        open_palette(&mut model);
        update(&mut model, PaletteMessage::Next);
        assert_eq!(model.palette.as_ref().unwrap().selected, 1);

        type_filter(&mut model, "q");
        assert_eq!(model.palette.as_ref().unwrap().selected, 0);
    }

    #[test]
    fn selection_wraps_in_both_directions() {
        let mut model = Model::default();
        open_palette(&mut model);
        let count = entries(&model).len();

        update(&mut model, PaletteMessage::Previous);
        assert_eq!(model.palette.as_ref().unwrap().selected, count - 1);

        update(&mut model, PaletteMessage::Next);
        assert_eq!(model.palette.as_ref().unwrap().selected, 0);
    }

    #[test]
    fn confirm_dispatches_selected_command_and_closes() {
        let mut model = Model::default();
        open_palette(&mut model);
        type_filter(&mut model, "quit");

        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(matches!(confirmed, Some(Message::Quit)));
        assert!(model.palette.is_none());
    }

    #[test]
    fn confirm_on_greyed_row_is_inert() {
        let mut model = Model::default();
        open_palette(&mut model);
        // No envelope is selected, so every envelope command is greyed.
        type_filter(&mut model, "reply all");

        assert_eq!(entry_ids(&model), ["reply-all"]);
        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(confirmed.is_none());
        assert!(
            model.palette.is_some(),
            "an inert confirm must keep the palette open"
        );
    }

    #[test]
    fn tab_completes_highlighted_id_into_filter() {
        let mut model = Model::default();
        open_palette(&mut model);
        type_filter(&mut model, "relo");

        update(&mut model, PaletteMessage::Complete);
        let palette = model.palette.as_ref().unwrap();
        assert_eq!(palette.input.value(), "reload-mailboxes");

        let confirmed = update(&mut model, PaletteMessage::Confirm);
        assert!(matches!(confirmed, Some(Message::LoadMailboxes)));
    }

    #[test]
    fn tab_completed_greyed_command_stays_highlighted() {
        let mut model = Model::default();
        open_palette(&mut model);
        type_filter(&mut model, "forward");

        update(&mut model, PaletteMessage::Complete);
        let palette = model.palette.as_ref().unwrap();
        let highlighted = &entries(&model)[palette.selected];
        assert_eq!(highlighted.command.id, "forward");
    }

    #[test]
    fn composer_commands_surface_once_composer_opens() {
        let mut model = Model {
            bottom_panel: BottomPanel::Compose,
            ..Model::default()
        };
        open_palette(&mut model);
        type_filter(&mut model, "send");

        let listed = entries(&model);
        assert_eq!(listed[0].command.id, "send");
        assert!(listed[0].is_available);
    }
}
