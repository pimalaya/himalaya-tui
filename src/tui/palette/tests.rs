use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
fn tab_completes_highlighted_name_into_filter() {
    let mut model = Model::default();
    open_palette(&mut model);
    type_filter(&mut model, "relo");

    update(&mut model, PaletteMessage::Complete);
    let palette = model.palette.as_ref().unwrap();
    assert_eq!(palette.input.value(), "Reload mailboxes");

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
