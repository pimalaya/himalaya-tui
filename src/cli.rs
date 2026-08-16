//! Clap-driven CLI surface and the bridge into the TUI: [`Cli::try_into_tui_model`]
//! turns parsed flags + on-disk config (or the wizard) into a ready-to-run
//! [`Model`], applying CLI overrides last.

use std::{env::temp_dir, fs::File, path::PathBuf, time::Instant};

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use edtui::{EditorState, Lines};
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

#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
use crate::wizard;
use crate::{
    config::{AccountConfig, Config},
    shared::client::EmailClient,
    tui::{
        model::{BottomPanel, Keybinds, Message, Model, Panel},
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

    /// Email address to discover a throwaway account from.
    ///
    /// Passing one skips the account lookup entirely and opens the
    /// account discovered from this value, so it is the quickest way to
    /// try a server without touching your configuration. A server URL
    /// and a local folder path work here too, the address being what
    /// this is reached for. The rest of the file (theme, signature,
    /// keybindings) still applies; `--no-config` drops that too.
    /// Omitting it and `--account` alike opens the default account.
    #[arg(value_name = "EMAIL", conflicts_with = "account")]
    pub seed: Option<String>,

    /// Account to open, as named in the configuration file.
    ///
    /// Defaults to the account flagged `default = true`. A name the
    /// file cannot answer, because it holds no such account or because
    /// there is no file at all, is an error rather than a fallback to
    /// the wizard: use the positional argument for that.
    #[arg(long, short, value_name = "NAME")]
    #[arg(conflicts_with_all = ["seed", "no_config"])]
    pub account: Option<String>,

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
    /// the run warns and falls back to the wizard, which builds an
    /// account in memory for this session only. Other paths are
    /// merged with the first one, which allows you to separate your
    /// public config from your private(s) one(s). Multiple paths can
    /// also be provided by delimiting them with `:` (like `$PATH` in
    /// a POSIX shell).
    #[arg(long = "config", short, global = true, env = "HIMALAYA_CONFIG")]
    #[arg(value_name = "PATH", value_parser = path_parser, value_delimiter = ':')]
    pub config_paths: Vec<PathBuf>,
    /// Skip the configuration file entirely and run the wizard.
    ///
    /// Useful when a config already exists on disk but you want a
    /// throwaway, in-memory account for this run (e.g. to hand the TUI
    /// off to someone else without exposing your stored credentials).
    /// Unlike the positional argument, this drops the whole file, theme
    /// and signature included. The wizard never writes to disk;
    /// `--config` and `HIMALAYA_CONFIG` are ignored when this flag is
    /// set, and no warning is raised since the wizard was asked for.
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

        // NOTE: a seed and `--no-config` ask for the wizard outright, so
        // the account lookup is skipped and nothing is warned about. The
        // wizard also runs when that lookup finds nothing, but there it
        // is a mistake the user can fix, and `wizard_reason` names which.
        let asked_for_wizard = self.no_config || self.seed.is_some();
        let mut wizard_reason = None;

        // NOTE: a seed keeps the file for its globals (theme, signature,
        // keybindings) and only swaps the account out; `--no-config`
        // drops the file whole.
        let loaded = if self.no_config {
            None
        } else {
            let loaded = Config::from_paths_or_default(&self.config_paths)?;

            if loaded.is_none() && !asked_for_wizard {
                let path = Config::target_path(&self.config_paths)?;

                // NOTE: `-a` addresses the file and nothing else, so a
                // file that is not there cannot answer it. Falling back
                // would silently open something other than what was
                // asked for.
                if let Some(name) = &self.account {
                    bail!(
                        "No account `{name}`: there is no configuration file at {}",
                        path.display()
                    );
                }

                wizard_reason = Some(format!(
                    "No configuration file at {}, falling back to an in-memory account",
                    path.display()
                ));
            }

            loaded
        };

        let mut account_name = self
            .seed
            .clone()
            .unwrap_or_else(|| String::from("unspecified"));
        let mut display_name = None;
        let mut signature = String::new();
        let mut keybinds_config = None;
        let mut theme = Theme::default();

        let mut account = None;
        if let Some(mut config) = loaded {
            display_name = config.display_name.take();
            signature = config.signature.take().unwrap_or_default();
            keybinds_config = config.keybinds.take();
            theme = Theme::resolve(&config.theme);

            if !asked_for_wizard {
                match config.take_account(self.account.as_deref())? {
                    Some((name, cfg)) => {
                        account_name = name;
                        account = Some(cfg);
                    }
                    None => {
                        wizard_reason = Some(String::from(
                            "Configuration file carries no default account, falling back to an in-memory account",
                        ));
                    }
                }
            }
        }

        let mut account_config = match account {
            Some(account) => account,
            // No stored account to run on, so the wizard builds one in
            // memory for this session. It is not the CLI's wizard: it
            // writes nothing and proposes no config entry. The seed
            // feeds it when one was given, otherwise it prompts.
            None => {
                match wizard_reason {
                    Some(reason) => spinner.failure(reason),
                    None => spinner.clear(),
                }
                let account = run_wizard(self.seed.as_deref(), self.from.as_deref())?;
                spinner = Spinner::start("Loading…");
                account
            }
        };

        let from = account_config.from.clone();
        let from_name = account_config.from_name.take().or(display_name);
        let signature = account_config.signature.take().unwrap_or(signature);
        let keybinds = self.keybinds.or(keybinds_config);

        let client = EmailClient::new(account_config)?;

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
