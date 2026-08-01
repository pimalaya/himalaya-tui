//! Clap-driven CLI surface and the bridge into the TUI: [`Cli::try_into_tui_model`]
//! turns parsed flags + on-disk config (or the wizard) into a ready-to-run
//! [`Model`], applying CLI overrides last.

use std::{env::temp_dir, fs::File, path::PathBuf};

use anyhow::Result;
#[cfg(not(all(feature = "imap", feature = "smtp", feature = "jmap")))]
use anyhow::bail;
use clap::{CommandFactory, Parser, Subcommand};
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

#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
use crate::wizard;
use crate::{
    config::{AccountConfig, Config, DEFAULT_PALETTE_KEY},
    session::{self, SessionSource},
    tui::{
        model::{Keybinds, Message, Model},
        theme::Theme,
        update,
    },
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

        let mut globals = TuiGlobals::default();
        let mut configured_account = None;
        if let Some(mut config) = loaded {
            globals = take_globals(&mut config);
            configured_account = config.take_account(self.account_or_server.as_deref())?;
        }

        // `config_source` stays `None` for wizard-built accounts even
        // when a config file exists: switching away from an in-memory
        // account would lose it irrecoverably.
        let (account_name, account, config_source) = match configured_account {
            Some((name, account)) => (name, account, Some(self.config_paths.clone())),
            // No matching account (no config, or the config carries no
            // such account and no default): fall back to the wizard,
            // seeding it with the positional argument (an email, server
            // or URI) when one was given, otherwise prompting for one.
            None => {
                spinner.clear();
                let account = run_wizard(self.account_or_server.as_deref(), self.from.as_deref())?;
                spinner = Spinner::start("Loading…");
                (String::from("unspecified"), account, None)
            }
        };

        let session = SessionSource {
            account_name,
            account,
            display_name: globals.display_name,
            signature: globals.signature,
            account_names: globals.account_names,
        }
        .open()?;

        let keybinds = self.keybinds.or(globals.keybinds).unwrap_or_default();

        let mut model = Model {
            editor_handler: keybinds.editor_handler(),
            keybinds,
            palette_key: globals.palette_key,
            theme: globals.theme,
            ..Model::from_session(session, config_source)
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

/// Top-level config fields consumed at startup: UI settings plus the
/// global identity fallbacks handed to [`SessionSource`].
struct TuiGlobals {
    display_name: Option<String>,
    signature: String,
    keybinds: Option<Keybinds>,
    palette_key: char,
    theme: Theme,
    account_names: Vec<String>,
}

impl Default for TuiGlobals {
    fn default() -> Self {
        Self {
            display_name: None,
            signature: String::new(),
            keybinds: None,
            palette_key: DEFAULT_PALETTE_KEY,
            theme: Theme::default(),
            account_names: Vec::new(),
        }
    }
}

fn take_globals(config: &mut Config) -> TuiGlobals {
    TuiGlobals {
        display_name: config.display_name.take(),
        signature: config.signature.take().unwrap_or_default(),
        keybinds: config.keybinds.take(),
        palette_key: config.palette_key.take().unwrap_or(DEFAULT_PALETTE_KEY),
        theme: Theme::resolve(&config.theme),
        account_names: session::sorted_account_names(config),
    }
}

/// Runs the interactive setup wizard used when no configuration file is
/// found. The wizard discovers IMAP/SMTP/JMAP accounts, so it is only
/// compiled when all three backends are enabled; other builds require a
/// configuration file.
#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
fn run_wizard(seed: Option<&str>, from: Option<&str>) -> Result<AccountConfig> {
    match seed {
        Some(seed) => wizard::discover::run_with_input(seed, from),
        None => wizard::discover::run(from),
    }
}

#[cfg(not(all(feature = "imap", feature = "smtp", feature = "jmap")))]
fn run_wizard(_seed: Option<&str>, _from: Option<&str>) -> Result<AccountConfig> {
    bail!(
        "The setup wizard requires the imap, smtp and jmap features; \
         pass a configuration file with --config instead"
    )
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
