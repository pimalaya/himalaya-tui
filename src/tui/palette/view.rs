//! Palette overlay rendering on the shared filter-picker chrome.

use ratatui::{
    Frame,
    text::{Line, Span},
    widgets::{List, ListItem},
};

use crate::tui::{
    model::Model,
    palette::{self, PALETTE_VISIBLE, PaletteEntry},
    view::{aligned_row, render_filter_overlay},
};

const TITLE: &str = " Command Palette ";

pub fn render(frame: &mut Frame, model: &Model) {
    let Some(state) = &model.palette else { return };

    let inner = render_filter_overlay(frame, &model.theme, TITLE, &state.input, PALETTE_VISIBLE);

    // Sliding window keeping the selection visible; a pure function
    // of the selection so no scroll state needs storing.
    let offset = state.selected.saturating_sub(PALETTE_VISIBLE - 1);

    let items: Vec<ListItem> = palette::entries(model)
        .iter()
        .enumerate()
        .skip(offset)
        .take(PALETTE_VISIBLE)
        .map(|(i, entry)| result_row(entry, model, i == state.selected, inner.width))
        .collect();

    frame.render_widget(List::new(items), inner);
}

fn result_row(
    entry: &PaletteEntry,
    model: &Model,
    is_selected: bool,
    width: u16,
) -> ListItem<'static> {
    let style = if is_selected {
        model.theme.cursor
    } else if entry.is_available {
        model.theme.message_body
    } else {
        model.theme.border_inactive
    };

    let hint = entry.command.hint(model.keybinds).unwrap_or_default();
    let row = aligned_row(entry.command.name, &hint, is_selected, width);

    ListItem::new(Line::from(Span::styled(row, style)))
}
