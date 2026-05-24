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

use clap::ValueEnum;
use edtui::EditorEventHandler;
use serde::{Deserialize, Serialize};

/// Keybinding flavor applied to the in-app composer.
///
/// Mirrors `edtui::EditorEventHandler::{vim_mode, emacs_mode}` and is
/// shared between the CLI flag, the TOML config and the `App` state
/// so the event loop can also gate its own intercepts (notably
/// `Ctrl-e`, which collides with Emacs' "move to end of line").
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Keybinds {
    #[default]
    Vim,
    Emacs,
}

impl Keybinds {
    /// Builds the edtui event handler that matches this flavor.
    pub fn editor_handler(self) -> EditorEventHandler {
        match self {
            Self::Vim => EditorEventHandler::vim_mode(),
            Self::Emacs => EditorEventHandler::emacs_mode(),
        }
    }
}
