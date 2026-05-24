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

//! Library crate backing the `himalaya-tui` binary.
//!
//! The binary is a thin shell that wires [`cli`] flag parsing and
//! [`config`] resolution into an [`app::App`] state machine rendered
//! by the [`ui`] module. The optional [`wizard`] runs first-time
//! discovery when no configuration file exists or `--no-config` is
//! passed. [`theme`] holds the resolved color theme and [`themes`]
//! ships the built-in presets.

pub mod app;
pub mod cli;
pub mod config;
pub mod mime;
pub mod runtime;
pub mod theme;
pub mod themes;
pub mod ui;
#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
pub mod wizard;
