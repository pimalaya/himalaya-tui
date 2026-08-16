//! # himalaya-tui
//!
//! TUI to manage emails. himalaya-tui is an application, the top layer
//! of the Pimalaya stack: it writes no protocol or storage logic of its
//! own and ships no library target, only this binary. It is a thin
//! shell driving the sans-I/O io-* libraries below it, consuming their
//! blocking `*Std` clients and rendering the results in a terminal.
//!
//! ## Backends and plumbing
//!
//! The network backends are io-imap, io-jmap and io-smtp; the local
//! storage backends are io-maildir and io-m2dir. Account discovery comes
//! from io-pim-discovery (Mozilla autoconfig, PACC, RFC 6186 SRV). The
//! terminal, prompt and wizard primitives, the TOML config loading and
//! the blocking stream runtime come from pimalaya-cli, pimalaya-config
//! and pimalaya-stream; message composition uses mml. Every backend sits
//! behind its own cargo feature, so a build ships only the protocols it
//! needs.
//!
//! ## Shared client and backend selection
//!
//! The TUI runs over a local [`shared::client`] `EmailClient` that owns
//! one `BackendClient` enum variant per compiled-in backend: the first
//! configured storage backend (local before network), plus an optional
//! SMTP transport for storage backends that cannot send (IMAP, Maildir,
//! m2dir), connected lazily on the first send. Each operation resolves
//! its mailbox argument to the backend-native id, then matches the
//! active backend and calls its per-protocol `backend.rs` adapter,
//! which converts io-* results into the TUI's own [`email`] shared
//! types (Envelope, Mailbox, Flag, Address).
//!
//! ## Terminal interface
//!
//! The interface follows the Elm Architecture (see [`tui`]): [`tui::model`]
//! owns all state and the `Message` enum, [`tui::update`] is the single
//! side-effecting transition function, [`tui::view`] renders the
//! three-pane layout (mailboxes, envelopes, message body or composer),
//! and [`tui::app`] drives the poll/update/redraw loop. The in-app
//! composer is powered by edtui with a system-editor handoff, and drafts
//! are written in MML then compiled to MIME on send.
//!
//! ## Startup
//!
//! [`main`] parses the CLI flags, runs any auxiliary subcommand
//! (completions, manuals), otherwise builds the [`tui::model::Model`]
//! and hands it to [`tui::app::run`].
//!
//! The account it runs on comes from the configuration file, which the
//! himalaya CLI authors and the TUI only reads: there is no `configure`
//! command here and nothing is ever written to disk. When that file
//! resolves no account, [`wizard`] fills the gap with a throwaway
//! account that lives for the session alone.

mod cli;
mod config;
mod email;
#[cfg(feature = "imap")]
mod imap;
#[cfg(feature = "jmap")]
mod jmap;
#[cfg(feature = "m2dir")]
mod m2dir;
#[cfg(feature = "maildir")]
mod maildir;
mod shared;
#[cfg(feature = "smtp")]
mod smtp;
mod tui;
#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
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
