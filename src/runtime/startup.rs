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

//! Pre-TUI startup: resolves the account config (from disk or the
//! wizard) and assembles the unified [`EmailClientStd`] before the
//! alternate screen takes over.

use std::path::PathBuf;

use anyhow::{Result, anyhow};
use io_email::client::EmailClientStd;
use pimalaya_cli::spinner::Spinner;
use pimalaya_config::toml::TomlConfig;

use crate::{
    app::{keybinds::Keybinds, state::App},
    config::{AccountConfig, Config},
    runtime::events::load_envelopes,
    theme::Theme,
};

#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
use crate::wizard;

/// Loads an account config from disk when one exists at the standard
/// paths (or `$HIMALAYA_CONFIG`), otherwise runs the wizard to build
/// one in memory. The wizard never writes to disk; users who want to
/// skip it should create their own config file.
///
/// `account_or_seed` carries the CLI positional. When a config is
/// found, it is matched against the `[accounts]` table; otherwise it
/// is fed to the wizard as an email/URL/domain seed. When `no_config`
/// is set, the on-disk lookup is bypassed entirely and the wizard
/// runs unconditionally. `from` (the CLI `--from` flag) is forwarded
/// to the wizard to prefill SASL/JMAP login prompts.
pub fn load_then_connect(
    config_paths: &[PathBuf],
    account_or_seed: Option<&str>,
    no_config: bool,
    from: Option<&str>,
    keybinds_cli: Option<Keybinds>,
) -> Result<(App, EmailClientStd)> {
    let loaded = if no_config {
        None
    } else {
        Config::from_paths_or_default(config_paths)?
    };

    let (name, mut account_config, display_name, signature, keybinds_config, theme) = match loaded {
        Some(mut config) => {
            let display = config.display_name.take();
            let sig = config.signature.take().unwrap_or_default();
            let keybinds = config.keybinds;
            let theme = Theme::resolve(&config.theme);
            let (name, account) = config
                .take_account(account_or_seed)?
                .ok_or_else(|| anyhow!("Account not found"))?;
            (name, account, display, sig, keybinds, theme)
        }
        None => {
            let account = run_wizard(account_or_seed, from)?;
            (
                "default".to_string(),
                account,
                None,
                String::new(),
                None,
                Theme::default(),
            )
        }
    };

    let from = account_config.from.clone();
    let from_name = account_config.from_name.take().or(display_name);
    let signature = account_config.signature.take().unwrap_or(signature);
    let smtp_config = account_config.smtp.clone();

    // CLI > config; `None` falls back to the composer's default Vim
    // handler. Top-level navigation no longer depends on this value:
    // Vim and Emacs aliases are always active alongside the universal
    // keys.
    let keybinds = keybinds_cli.or(keybinds_config);

    // Past this point everything is blocking I/O with no further
    // interactive prompts, so the spinner is safe until just before
    // the TUI grabs the alternate screen.
    let spinner = Spinner::start("Building client...");

    let mut client = match build_client(account_config) {
        Ok(client) => client,
        Err(err) => {
            spinner.failure(format!("{err}"));
            return Err(err);
        }
    };

    let mut app = App::new(
        name,
        from,
        from_name,
        signature,
        smtp_config,
        keybinds,
        theme,
    );

    spinner.set_message("Fetching mailboxes...");
    match client.list_mailboxes(false) {
        Ok(mailboxes) => app.set_mailboxes(mailboxes),
        Err(err) => {
            // Hand the error to the TUI status bar so the user can
            // still see and reach the rest of the interface.
            app.set_status(format!("Error: {err}"));
            spinner.clear();
            return Ok((app, client));
        }
    }

    if let Some(name) = app.selected_mailbox_name() {
        spinner.set_message(format!("Fetching envelopes from {name}..."));
    }
    load_envelopes(&mut app, &mut client);

    spinner.clear();

    Ok((app, client))
}

#[cfg(all(feature = "imap", feature = "smtp", feature = "jmap"))]
fn run_wizard(seed: Option<&str>, from: Option<&str>) -> Result<AccountConfig> {
    match seed {
        Some(seed) => wizard::discover::run_with_input(seed, from),
        None => wizard::discover::run(from),
    }
}

#[cfg(not(all(feature = "imap", feature = "smtp", feature = "jmap")))]
fn run_wizard(_seed: Option<&str>, _from: Option<&str>) -> Result<AccountConfig> {
    Err(anyhow!(
        "No config found and the wizard requires imap+smtp+jmap features."
    ))
}

/// Registers each configured backend on the unified client. Order is
/// JMAP -> IMAP -> Maildir for storage (richest first), then SMTP last
/// so JMAP-only accounts keep sending via JMAP and IMAP/Maildir
/// accounts pick up SMTP for sending.
#[allow(unused_variables, unused_mut)]
fn build_client(account_config: AccountConfig) -> Result<EmailClientStd> {
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
            Err(err) => log::warn!("SMTP backend disabled: {err}. Sending will be unavailable."),
        }
    }

    if !configured {
        anyhow::bail!("Wizard produced no usable backend");
    }

    Ok(client)
}
