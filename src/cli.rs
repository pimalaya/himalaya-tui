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

//! Clap-driven CLI surface and the bridge into the TUI: [`Cli::try_into_tui_model`]
//! turns parsed flags + on-disk config (or the wizard) into a ready-to-run
//! [`Model`], applying CLI overrides last.

use std::{env::temp_dir, fs::File, path::PathBuf, time::Instant};

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use edtui::{EditorState, Lines};
use io_email::client::EmailClientStd;
use pimalaya_cli::{
    clap::{
        args::{JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
        parsers::path_parser,
    },
    long_version,
    printer::Printer,
    spinner::Spinner,
};
use pimalaya_config::toml::TomlConfig;
use simplelog::WriteLogger;
use tui_input::Input;

use crate::{
    config::Config,
    tui::{
        model::{BottomPanel, Keybinds, Message, Model, Panel},
        theme::Theme,
        update,
    },
    wizard,
};

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Account name, or anything that can be used by the wizard to discover
    /// your account (URL, domain, email etc).
    #[arg(name = "account_name", value_name = "ACCOUNT-OR-SERVER")]
    pub account_or_server: Option<String>,

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

impl Cli {
    pub fn try_into_tui_model(self) -> Result<Model> {
        let mut spinner = Spinner::start("Loading…");

        WriteLogger::init(
            self.log.level.unwrap_or_default().into(),
            Default::default(),
            File::create(match self.log.file {
                Some(path) => path,
                None => temp_dir().join("himalaya-tui.log"),
            })?,
        )?;

        let loaded = if self.no_config {
            None
        } else {
            Config::from_paths_or_default(&self.config_paths)?
        };

        let mut account_name = String::from("unspecified");
        let mut display_name = None;
        let mut signature = String::new();
        let mut keybinds_config = None;
        let mut theme = Theme::default();

        let mut account_config = if let Some(mut config) = loaded {
            display_name = config.display_name.take();
            signature = config.signature.take().unwrap_or_default();
            keybinds_config = config.keybinds.take();
            theme = Theme::resolve(&config.theme);
            match config.take_account(self.account_or_server.as_deref())? {
                None => bail!("Account not found"),
                Some((name, account)) => {
                    account_name = name;
                    account
                }
            }
        } else {
            spinner.clear();
            let account = match self.account_or_server.as_deref() {
                Some(seed) => wizard::discover::run_with_input(seed, self.from.as_deref()),
                None => wizard::discover::run(self.from.as_deref()),
            }?;
            spinner = Spinner::start("Loading…");
            account
        };

        let from = account_config.from.clone();
        let from_name = account_config.from_name.take().or(display_name);
        let signature = account_config.signature.take().unwrap_or(signature);
        let keybinds = self.keybinds.or(keybinds_config);

        let mut client = EmailClientStd::new();
        let mut configured = false;

        #[cfg(feature = "jmap")]
        if let Some(jmap_cfg) = account_config.jmap {
            client = client.with_jmap(jmap_cfg.into_client()?);
            configured = true;
        }

        #[cfg(feature = "imap")]
        if let Some(imap_cfg) = account_config.imap {
            client = client.with_imap(imap_cfg.into_client()?);
            configured = true;
        }

        #[cfg(feature = "maildir")]
        if let Some(maildir_cfg) = account_config.maildir {
            client = client.with_maildir(maildir_cfg.into_client());
            configured = true;
        }

        #[cfg(feature = "m2dir")]
        if let Some(m2dir_cfg) = account_config.m2dir {
            client = client.with_m2dir(m2dir_cfg.into_client());
            configured = true;
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp_cfg) = account_config.smtp {
            match smtp_cfg.into_client() {
                Ok(smtp) => client = client.with_smtp(smtp),
                Err(err) => {
                    log::warn!("SMTP backend disabled: {err}. Sending will be unavailable.")
                }
            }
        }

        if !configured {
            bail!("Wizard produced no usable backend");
        }

        let mut model = Model {
            running: true,
            active_panel: Panel::Mailboxes,
            mailboxes: Vec::new(),
            mailbox_index: 0,
            mailbox_offset: 0,
            mailbox_filter: Input::default(),
            envelopes: Vec::new(),
            envelope_index: 0,
            envelope_offset: 0,
            envelope_page: 0,
            envelope_page_size: 50,
            envelope_total: 0,
            selected_mailbox: None,
            account_name,
            from,
            from_name,
            signature,
            status_message: None,
            bottom_panel: BottomPanel::None,
            message_content: None,
            message_scroll: 0,
            editor_state: EditorState::new(Lines::from("")),
            editor_handler: keybinds.unwrap_or_default().editor_handler(),
            dialog: None,
            dialog_index: 0,
            keybinds,
            theme,
            client,
            last_activity: Instant::now(),
        };

        if let Some(from) = self.from {
            model.from = Some(from);
        }

        if let Some(from_name) = self.from_name {
            model.from_name = Some(from_name);
        }

        update::apply_all(&mut model, Some(Message::Initialize));
        spinner.clear();

        Ok(model)
    }
}

/// Auxiliary subcommands. When none is given, the binary launches the
/// TUI as usual.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate shell completion scripts.
    Completions(CompletionCommand),
    /// Generate man pages.
    Manuals(ManualCommand),
}

impl Command {
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        match self {
            Self::Completions(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manuals(cmd) => cmd.execute(printer, Cli::command()),
        }
    }
}
