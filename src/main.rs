//! Binary entry point: parse CLI flags, run any auxiliary subcommand
//! (completions, manuals), otherwise build the [`tui::model::Model`]
//! from config or wizard and hand it to [`tui::app::run`].

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
mod session;
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
