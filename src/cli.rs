//! # Parser
//!
//! Top-level CLI parser and the bridge into the interface:
//! [`Cli::try_into_tui_model`] turns the parsed flags and the on-disk
//! configuration, or the wizard, into a ready-to-run [`Model`].

use std::{
    env::temp_dir,
    fs::File,
    io::{IsTerminal, stdin},
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Result, bail};
use clap::{CommandFactory, Parser, Subcommand};
use edtui::{EditorState, Lines};
use pimalaya_cli::{
    clap::{
        args::{JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
        parsers::path_parser,
    },
    footer, long_version,
    printer::Printer,
    prompt,
    spinner::Spinner,
};
use pimalaya_config::toml::TomlConfig;
use ratatui::crossterm::terminal;
use simplelog::WriteLogger;
use tui_input::Input;

use crate::{
    config::{AccountConfig, Config},
    shared::client::EmailClient,
    tui::{
        model::{BottomPanel, Keybinds, Message, Model, Panel},
        theme::Theme,
        update, view,
    },
    wizard,
};

/// RFC 3676 section 4.3 signature separator.
///
/// Written before the signature when neither the account nor the global
/// configuration names one, matching the himalaya CLI's own default.
const DEFAULT_SIGNATURE_DELIM: &str = "-- \n";

#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(after_help = footer!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    /// The command to run; a bare `himalaya-tui` opens the interface.
    #[command(subcommand)]
    pub command: Option<Command>,
    /// Email address to configure an account from.
    ///
    /// Passing one skips the account lookup and runs the wizard on this
    /// value, answering its first prompt: the quickest way to try a
    /// server. A server URL and a local folder path work here too. The
    /// rest of the file (theme, signature, keybindings) still applies,
    /// and `--no-config` drops that too. Omitting both this and
    /// `--account` opens the default account.
    #[arg(value_name = "EMAIL", conflicts_with = "account")]
    pub seed: Option<String>,
    /// Account to open, as named in the configuration file.
    ///
    /// Defaults to the account flagged `default = true`. A name the file
    /// cannot answer, holding no such account or being absent
    /// altogether, is an error rather than a fallback to the wizard: use
    /// the positional argument for that.
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
    /// Falls back to the top-level `keybinds` field of the configuration
    /// when omitted, then to Vim.
    #[arg(long, value_name = "FLAVOR", value_enum)]
    pub keybinds: Option<Keybinds>,
    /// Override the default configuration file path.
    ///
    /// Paths are shell-expanded then canonicalized. Multiple ones may be
    /// delimited by `:` (like `$PATH` in a POSIX shell) and are merged
    /// onto the first, which is how a public configuration stays apart
    /// from private ones. When the first names no valid file, the run
    /// falls back to the wizard, which offers to write one there.
    #[arg(long = "config", short, global = true, env = "HIMALAYA_CONFIG")]
    #[arg(value_name = "PATH", value_parser = path_parser, value_delimiter = ':')]
    pub config_paths: Vec<PathBuf>,
    /// Skip the configuration file entirely and run the wizard.
    ///
    /// Useful when a configuration exists on disk but you want another
    /// account for this run. Unlike the positional argument, this drops
    /// the whole file, theme and signature included, and prints no
    /// welcome. The file is not read, but `--config` and
    /// `HIMALAYA_CONFIG` still name the one the wizard offers to file
    /// its account in.
    #[arg(long = "no-config")]
    pub no_config: bool,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

impl Cli {
    /// Builds the interface model this invocation asks for.
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

        // NOTE: a run that asked for the wizard goes straight to its
        // prompts. One that falls back to it says why: `welcome` carries
        // the path a missing file was looked for at, `wizard_reason` the
        // mistake the user can fix.
        let asked_for_wizard = self.no_config || self.seed.is_some();
        let mut welcome = None;
        let mut wizard_reason = None;

        // NOTE: a seed keeps the file for its globals (theme, signature,
        // keybindings) and only swaps the account out.
        let loaded = if self.no_config {
            None
        } else {
            let loaded = Config::from_paths_or_default(&self.config_paths)?;

            if loaded.is_none() && !asked_for_wizard {
                let path = Config::target_path(&self.config_paths)?;

                if let Some(name) = &self.account {
                    bail!(
                        "No account `{name}`: there is no configuration file at {}",
                        path.display()
                    );
                }

                welcome = Some(path);
            }

            loaded
        };

        let mut display_name = None;
        let mut signature = String::new();
        let mut signature_delim = None;
        let mut keybinds_config = None;
        let mut theme = Theme::default();

        let mut account = None;
        if let Some(mut config) = loaded {
            display_name = config.display_name.take();
            signature = config.signature.take().unwrap_or_default();
            signature_delim = config.signature_delim.take();
            keybinds_config = config.keybinds.take();
            theme = Theme::resolve(&config.theme);

            if !asked_for_wizard {
                match config.take_account(self.account.as_deref())? {
                    Some(named) => account = Some(named),
                    None => {
                        wizard_reason = Some(String::from(
                            "Configuration file carries no default account, falling back to the wizard",
                        ));
                    }
                }
            }
        }

        let (account_name, mut account_config) = match account {
            Some(named) => named,
            None => {
                match wizard_reason {
                    Some(reason) => spinner.failure(reason),
                    None => spinner.clear(),
                }
                let named =
                    run_wizard(welcome.as_deref(), self.seed.as_deref(), &self.config_paths)?;
                spinner = Spinner::start("Loading…");
                named
            }
        };

        let from = account_config.from.clone();
        let from_name = account_config.from_name.take().or(display_name);
        let signature = signature_block(
            account_config.signature.take().unwrap_or(signature),
            account_config.signature_delim.take().or(signature_delim),
        );
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
            envelope_page_size: startup_page_size(),
            envelope_capacity: 0,
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

/// Envelopes the first page is fetched with, sized off the terminal.
///
/// There is no frame to measure yet, but the first render re-pages the
/// list when this missed, so the estimate only spares the session a
/// second listing. The main area is the terminal without its header and
/// status lines, and one too small to hold a row still asks for one.
fn startup_page_size() -> usize {
    let rows = terminal::size().map(|(_, rows)| rows).unwrap_or_default();

    view::envelope_capacity(rows.saturating_sub(2)).max(1)
}

/// Assembles the block the composer appends: separator, then signature.
///
/// Empty when the account declares no signature, so nothing is appended
/// at all. mml writes what it is given verbatim, so the separator is
/// resolved here rather than expected inside the configured value: one
/// file, one meaning, whichever binary composes.
fn signature_block(signature: String, delim: Option<String>) -> String {
    if signature.trim().is_empty() {
        return String::new();
    }

    let delim = delim.unwrap_or_else(|| DEFAULT_SIGNATURE_DELIM.to_string());

    format!("{delim}{}", signature.trim_end_matches('\n'))
}

/// Runs the setup wizard, returning the account to open under the name
/// it would be filed as.
///
/// `welcome` is set only when no configuration file was found, the one
/// case the wizard introduces itself in. Declining leaves the run with
/// nothing to open, so it stops there rather than opening onto no
/// mailbox. A run that asked for the wizard skips both.
fn run_wizard(
    welcome: Option<&Path>,
    seed: Option<&str>,
    config_paths: &[PathBuf],
) -> Result<(String, AccountConfig)> {
    // NOTE: nobody answers a prompt without a terminal, and the wizard
    // fails on that condition naming it.
    if let Some(path) = welcome.filter(|_| stdin().is_terminal()) {
        wizard::configure::print_welcome(path);

        if !prompt::bool("Create a configuration with a default account?", true)? {
            bail!(
                "No account to open: write a configuration at {} by hand, \
                 starting from the documented sample:\n  {}",
                path.display(),
                wizard::discover::CONFIG_SAMPLE_URL,
            );
        }
    }

    wizard::configure::run(seed, config_paths)
}

/// Auxiliary subcommands, the interface running when none is given.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate shell completion scripts.
    #[command(alias = "completions")]
    Completion(CompletionCommand),
    /// Generate man pages.
    #[command(alias = "manuals")]
    Manual(ManualCommand),
}

impl Command {
    /// Runs the subcommand and prints its output.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        match self {
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_signature_block_carries_its_separator() {
        assert_eq!(
            signature_block("Alice".to_string(), None),
            "-- \nAlice",
            "an unset delimiter falls back to the RFC separator",
        );

        assert_eq!(
            signature_block("Alice\n".to_string(), Some("~~~\n".to_string())),
            "~~~\nAlice",
            "the delimiter is written verbatim, its own newline included",
        );
    }

    #[test]
    fn no_signature_means_no_block_at_all() {
        assert_eq!(signature_block(String::new(), None), "");
        // NOTE: a delimiter alone is not a signature, or mml would
        // append a bare `-- ` to every draft.
        assert_eq!(
            signature_block("  \n".to_string(), Some("-- \n".to_string())),
            "",
        );
    }
}
