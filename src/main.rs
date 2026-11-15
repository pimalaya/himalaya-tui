//! # himalaya-tui
//!
//! TUI to manage emails, a frontend of the himalaya CLI. It writes no
//! protocol or storage logic of its own and ships no library target, only
//! this binary.
//!
//! ## Himalaya underneath
//!
//! The TUI links himalaya as a library, pinned by git revision, and runs
//! himalaya's own commands in-process over one session kept for the whole
//! run, as himalaya's REPL and MCP server do ([`client`]). The session
//! builds its client on the first command, keeps it, and drops it when a
//! command fails, the next one connecting again. Every backend himalaya
//! speaks is available, each behind the cargo feature of the same name
//! forwarded to himalaya's.
//!
//! A command's answer is read as its documented `--json` value, decoded
//! into the [`email`] types the interface renders: the coupling is the
//! CLI's versioned contract, never himalaya's Rust types.
//!
//! ## Configuration
//!
//! The configuration is himalaya's: the account, its backends, its
//! identity and its signature are resolved through himalaya's own types.
//! The TUI adds a few keys under its own `tui` tables ([`config`]): the
//! composer's keybindings, the contact command and the theme.
//!
//! ## Terminal interface
//!
//! The interface follows the Elm Architecture (see [`tui`]):
//! [`tui::model`] owns all state and the `Message` enum, [`tui::update`]
//! is the single side-effecting transition function, [`tui::view`]
//! renders the three panes, and [`tui::app`] drives the loop.
//!
//! The in-app composer is powered by edtui with a system-editor handoff,
//! and drafts are written in MML then compiled to MIME on send.
//!
//! Recipients complete from a command the user configures, the aerc
//! way, so the TUI holds no address book: [`contact`] locates the
//! fragment, runs the command on a thread the loop polls, and formats
//! the pick as an RFC 5322 mailbox.
//!
//! ## Startup
//!
//! [`main`] parses the CLI flags through pimalaya-cli, so a refused
//! command line answers as in himalaya, runs any standard subcommand
//! (completions, manuals), otherwise builds the [`tui::model::Model`],
//! loading the mailboxes and the first page before the interface takes
//! the terminal, and hands it to [`tui::app::run`]. With no configuration
//! it fails there with himalaya's error, naming the path and the sample.

mod cli;
mod client;
mod config;
mod contact;
mod email;
mod tui;

use himalaya::error::{self, ErrorCode};
use pimalaya_cli::{clap::parse, error::ErrorReport, printer::StdoutPrinter, session::claim_stdin};
use simd_cpu::SimdCpu;

use crate::{cli::Cli, tui::app};

fn main() {
    // NOTE: himalaya's base64 and quoted-printable codecs read the
    // installed CPU token, the portable baseline until one is installed.
    let _ = SimdCpu::install_detected();

    // NOTE: the terminal is the interface's, so a command that would read
    // its input from stdin is refused rather than reading it.
    claim_stdin();

    // A refused command line exits 2, the failure object coded
    // `usage-invalid` on stdout under `--json`, as in himalaya.
    let cli = parse::<Cli>();
    let mut printer = StdoutPrinter::new(&cli.json);

    if let Some(command) = cli.command {
        let result = command.execute(&mut printer).map_err(error::coded);
        return ErrorReport::<ErrorCode>::eval_coded(&mut printer, result);
    }

    // NOTE: an account the configuration cannot give, or a backend the
    // build leaves out, fails with himalaya's own code.
    let result = cli.try_into_tui_model().map_err(error::coded);
    let model = ErrorReport::<ErrorCode>::eval_coded(&mut printer, result);

    let result = app::run(model);
    ErrorReport::eval(&mut printer, result);
}
