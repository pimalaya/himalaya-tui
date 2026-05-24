// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026  soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use edtui::{EditorTheme, EditorView};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Widget},
};

use crate::app::{keybinds::Keybinds, panel::Panel, state::App};

use super::layout::get_border_style;

pub fn render_compose(frame: &mut Frame, app: &mut App, area: Rect) {
    // Vim binds both `Ctrl-e` (edtui's normal-mode shortcut) and
    // `Alt-e`; Emacs binds only `Alt-e` because `Ctrl-e` is taken by
    // "move to end of line".
    let editor_hint = match app.keybinds.unwrap_or_default() {
        Keybinds::Vim => "Ctrl-e or Alt-e: open in $EDITOR",
        Keybinds::Emacs => "Alt-e: open in $EDITOR",
    };
    let title = format!(" Compose (Esc: actions, {editor_hint}) ");

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(get_border_style(app, Panel::Compose));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let editor_theme = EditorTheme::default()
        .base(app.theme.compose_text)
        .cursor_style(app.theme.compose_cursor)
        .selection_style(app.theme.compose_selection)
        .hide_status_line();

    let buf = frame.buffer_mut();
    EditorView::new(&mut app.editor_state)
        .theme(editor_theme)
        .render(inner, buf);
}
