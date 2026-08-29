//! # himalaya-tui
//!
//! TUI to manage emails, the top layer of the Pimalaya stack: it writes
//! no protocol or storage logic of its own and ships no library target,
//! only this binary. It is a thin shell driving the sans-I/O io-*
//! libraries below it through their blocking `*Std` clients.
//!
//! ## Backends and plumbing
//!
//! The network backends are io-imap, io-jmap and io-smtp, the local
//! storage backend io-maildir. Account discovery comes from
//! io-pim-discovery, searching provider rules, PACC, Mozilla autoconfig,
//! RFC 6186 SRV and the RFC 8620 JMAP resolve in parallel.
//!
//! Terminal, prompt and wizard primitives, TOML loading and the blocking
//! stream runtime come from pimalaya-cli, pimalaya-config and
//! pimalaya-stream, message composition from mml. Every backend sits
//! behind its own cargo feature, so a build ships only what it needs.
//!
//! ## Shared client and backend selection
//!
//! The interface runs over a [`shared::client`] `EmailClient` owning one
//! `BackendClient` variant per compiled-in backend: the first configured
//! storage backend (local before network), plus an SMTP transport for
//! the storage backends that cannot send, connected on the first send.
//!
//! Each operation resolves its mailbox argument to the backend-native
//! id, then matches the active backend and calls its per-protocol
//! backend.rs adapter, which converts io-* results into the [`email`]
//! shared types (Envelope, Mailbox, Flag, Address).
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
//! ## Startup
//!
//! [`main`] parses the CLI flags, runs any auxiliary subcommand
//! (completions, manuals), otherwise builds the [`tui::model::Model`]
//! and hands it to [`tui::app::run`].
//!
//! The account it runs on comes from the configuration file, shared with
//! the himalaya CLI. When that file resolves no account, [`wizard`]
//! fills the gap with the CLI's own flow, prompt for prompt, offering on
//! the way out to file what it discovered.
//!
//! The account is opened either way, for the session alone when nothing
//! was written.

mod cli;
mod config;
mod email;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "jmap")]
mod jmap;
#[cfg(feature = "maildir")]
mod maildir;
mod shared;
#[cfg(feature = "smtp")]
mod smtp;
mod tui;
mod wizard;

use clap::Parser;
use pimalaya_cli::{error::ErrorReport, printer::StdoutPrinter};

use crate::{cli::Cli, tui::app};

fn main() {
    let cli = Cli::parse();
    let mut printer = StdoutPrinter::new(&cli.json);

    if let Some(command) = cli.command {
        let result = command.execute(&mut printer);
        return ErrorReport::eval(&mut printer, result);
    }

    let result = cli.try_into_tui_model();
    let model = ErrorReport::eval(&mut printer, result);

    let result = app::run(model);
    ErrorReport::eval(&mut printer, result);
}
