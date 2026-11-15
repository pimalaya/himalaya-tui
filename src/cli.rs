//! # Parser
//!
//! Top-level CLI parser and the bridge into the interface:
//! [`Cli::try_into_tui_model`] resolves the account through himalaya,
//! reads the TUI's own keys beside it and opens the session, then loads
//! the first screen before the interface takes the terminal.

use std::{env::temp_dir, fs::File, time::Instant};

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use edtui::{EditorState, Lines};
#[cfg(backend)]
use himalaya::account::context::Account;
use himalaya::{
    backend::Backend,
    cli::resolve_account,
    config::{AccountConfig, Config as HimalayaConfig},
};
use pimalaya_cli::{
    clap::{
        args::{AccountFlag, ConfigFlags, JsonFlag, LogFlags},
        commands::{completion::CompletionCommand, manual::ManualCommand},
    },
    footer, long_version,
    printer::Printer,
    spinner::Spinner,
};
use pimalaya_config::toml::TomlConfig;
use ratatui::crossterm::terminal;
use simplelog::WriteLogger;
use tui_input::Input;

use crate::{
    client::MailClient,
    config::Config,
    tui::{
        model::{BottomPanel, Keybinds, Message, Model, Panel},
        theme::Theme,
        update, view,
    },
};

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
    #[command(flatten)]
    pub config: ConfigFlags,
    #[command(flatten)]
    pub account: AccountFlag,
    /// Force a specific backend, as the himalaya CLI's `--backend` does.
    ///
    /// `auto`, the default, picks the first backend the account
    /// configures, local before network.
    #[arg(short, long, global = true, default_value_t)]
    pub backend: Backend,
    /// Override the From address used when sending or saving drafts.
    #[arg(long, value_name = "EMAIL")]
    pub from: Option<String>,
    /// Override the From display name used when sending or saving
    /// drafts.
    #[arg(long = "from-name", value_name = "NAME")]
    pub from_name: Option<String>,
    /// Keybinding flavor applied to the in-app composer.
    ///
    /// Falls back to the `tui.keybinds` field of the configuration when
    /// omitted, then to Vim.
    #[arg(long, value_name = "FLAVOR", value_enum)]
    pub keybinds: Option<Keybinds>,
    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

impl Cli {
    /// Builds the interface model this invocation asks for, its first
    /// screen loaded.
    ///
    /// Runs before the interface takes the terminal, so a secret command
    /// prompting there asks on the plain terminal.
    pub fn try_into_tui_model(self) -> Result<Model> {
        let spinner = Spinner::start("Loading…");
        let model = self.build_model();
        spinner.clear();
        model
    }

    fn build_model(self) -> Result<Model> {
        WriteLogger::init(
            self.log.level.unwrap_or_default().into(),
            Default::default(),
            File::create(match self.log.file {
                Some(path) => path,
                None => temp_dir().join("himalaya-tui.log"),
            })?,
        )?;

        let paths = self.config.paths;

        // NOTE: himalaya resolves the account, so a missing file or name
        // fails with the CLI's own error, before anything is drawn.
        let (config, account_name, account_config) =
            resolve_account(&paths, self.account.name.as_deref())?;
        let account = AccountIdentity::new(config, account_config)?;

        let Config { tui, mut accounts } = Config::load(&paths)?;
        let tui_account = accounts.remove(&account_name).unwrap_or_default().tui;

        let keybinds = self.keybinds.or(tui.keybinds);
        let contact_command = tui_account.contact_command.or(tui.contact_command);
        let client = MailClient::new(paths, Some(account_name.clone()), self.backend);

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
            envelope_total: None,
            envelope_has_next: false,
            envelope_end: None,
            marked: Vec::new(),
            selected_mailbox: None,
            account_name,
            from: self.from.or(account.email),
            from_name: self.from_name.or(account.display_name),
            signature: account.signature,
            status_message: None,
            deferred: None,
            bottom_panel: BottomPanel::None,
            message_content: None,
            message_scroll: 0,
            editor_state: EditorState::new(Lines::from("")),
            editor_handler: keybinds.unwrap_or_default().editor_handler(),
            dialog: None,
            dialog_index: 0,
            keybinds,
            contact_command,
            contact_complete_key: tui.contact_complete_key.unwrap_or_default(),
            contact_lookup: None,
            contact_completion: None,
            theme: Theme::resolve(&tui.theme),
            client,
            last_activity: Instant::now(),
        };

        update::apply_all(&mut model, Some(Message::Initialize));

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

/// What the composer takes from the account himalaya merges from the
/// configuration's global keys and the account's own.
#[cfg_attr(not(backend), allow(dead_code))]
struct AccountIdentity {
    /// The block the composer appends, see [`signature_block`].
    signature: String,
    /// The sender address.
    email: Option<String>,
    /// The name that address carries.
    display_name: Option<String>,
}

impl AccountIdentity {
    #[cfg(backend)]
    fn new(config: HimalayaConfig, account_config: AccountConfig) -> Result<Self> {
        let account = Account::from(config).merge(Account::from(account_config));
        let signature = signature_block(account.signature.as_deref(), account.signature_delim());

        Ok(Self {
            signature,
            email: account.email,
            display_name: account.display_name,
        })
    }

    /// A build with no backend reaches no mailbox: it refuses to start,
    /// naming the cargo features that give it one.
    #[cfg(not(backend))]
    fn new(_: HimalayaConfig, _: AccountConfig) -> Result<Self> {
        anyhow::bail!(
            "This himalaya-tui was built with no backend: build it with one of \
             the imap, smtp, jmap, gmail, msgraph, maildir, mbox or pimdir \
             cargo features"
        )
    }
}

/// Assembles the block the composer appends: separator, then signature.
///
/// Empty when the account declares no signature, so nothing is appended
/// at all. mml writes what it is given verbatim, so the separator is
/// resolved here rather than expected inside the configured value: one
/// file, one meaning, whichever binary composes.
#[cfg_attr(not(backend), allow(dead_code))]
fn signature_block(signature: Option<&str>, delim: &str) -> String {
    let Some(signature) = signature.filter(|signature| !signature.trim().is_empty()) else {
        return String::new();
    };

    format!("{delim}{}", signature.trim_end_matches('\n'))
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
            signature_block(Some("Alice"), "-- \n"),
            "-- \nAlice",
            "the RFC separator himalaya defaults to",
        );

        assert_eq!(
            signature_block(Some("Alice\n"), "~~~\n"),
            "~~~\nAlice",
            "the delimiter is written verbatim, its own newline included",
        );
    }

    #[test]
    fn no_signature_means_no_block_at_all() {
        assert_eq!(signature_block(None, "-- \n"), "");
        // NOTE: a delimiter alone is not a signature, or mml would
        // append a bare `-- ` to every draft.
        assert_eq!(signature_block(Some("  \n"), "-- \n"), "");
    }
}
