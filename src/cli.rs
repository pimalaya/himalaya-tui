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

//! Clap-driven command-line interface for the `himalaya-tui` binary.

use std::path::PathBuf;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
        parsers::path_parser,
    },
    printer::Printer,
};

use crate::app::keybinds::Keybinds;

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
pub struct HimalayaTui {
    #[command(subcommand)]
    pub command: Option<HimalayaTuiCommand>,

    /// Override the From address used when sending or saving drafts.
    #[arg(long, value_name = "EMAIL")]
    pub from: Option<String>,
    /// Override the From display name used when sending or saving
    /// drafts.
    #[arg(long = "from-name", value_name = "NAME")]
    pub from_name: Option<String>,

    /// Keybinding flavor applied to the in-app composer.
    ///
    /// When omitted, falls back to the top-level `keybinds` field in
    /// the TOML config (if present), otherwise to Vim.
    #[arg(long, value_name = "FLAVOR", value_enum)]
    pub keybinds: Option<Keybinds>,
    #[command(flatten)]
    pub account: AccountFlag,
    /// Override the default configuration file path.
    ///
    /// The given paths are shell-expanded then canonicalized (if
    /// applicable). If the first path does not point to a valid file,
    /// the wizard is run to build a config in memory. Other paths are
    /// merged with the first one, which allows you to separate your
    /// public config from your private(s) one(s). Multiple paths can
    /// also be provided by delimiting them with `:` (like `$PATH` in
    /// a POSIX shell).
    #[arg(long = "config", short, global = true, env = "HIMALAYA_CONFIG")]
    #[arg(value_name = "PATH", value_parser = path_parser, value_delimiter = ':')]
    pub config_paths: Vec<PathBuf>,
    /// Skip configuration file lookup and run the wizard.
    ///
    /// Useful when a config already exists on disk but you want a
    /// throwaway, in-memory account for this run (e.g. to try another
    /// server, or hand off the TUI to someone else without exposing
    /// your stored credentials). The wizard never writes to disk;
    /// `--config` and `HIMALAYA_CONFIG` are ignored when this flag is
    /// set.
    #[arg(long = "no-config")]
    pub no_config: bool,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

/// Auxiliary subcommands. When none is given, the binary launches the
/// TUI as usual.
#[derive(Debug, Subcommand)]
pub enum HimalayaTuiCommand {
    /// Generate shell completion scripts.
    Completions(CompletionCommand),
    /// Generate man pages.
    Manuals(ManualCommand),
}

impl HimalayaTuiCommand {
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        match self {
            Self::Completions(cmd) => cmd.execute(printer, HimalayaTui::command()),
            Self::Manuals(cmd) => cmd.execute(printer, HimalayaTui::command()),
        }
    }
}
