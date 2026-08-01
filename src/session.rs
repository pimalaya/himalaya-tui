//! Turns configuration into a live session in one place, so startup
//! ([`crate::cli`]) and the in-app account switcher derive an
//! account's identity identically.

use std::path::PathBuf;

use anyhow::{Context, Result};
use pimalaya_config::toml::TomlConfig;

use crate::{
    config::{AccountConfig, Config},
    shared::client::EmailClient,
};

/// A ready session for one account: its derived identity, an open
/// client, and the sibling account names feeding the switcher.
pub struct AccountSession {
    pub account_name: String,
    pub from: Option<String>,
    pub from_name: Option<String>,
    pub signature: String,
    pub account_names: Vec<String>,
    pub client: EmailClient,
}

/// Inputs for opening a session: one account plus the global config
/// fields that complete its identity.
pub struct SessionSource {
    pub account_name: String,
    pub account: AccountConfig,
    pub display_name: Option<String>,
    pub signature: String,
    pub account_names: Vec<String>,
}

impl SessionSource {
    /// Derives the identity (account fields first, globals as
    /// fallback) and opens the client connections.
    pub fn open(mut self) -> Result<AccountSession> {
        let from = self.account.from.clone();
        let from_name = self.account.from_name.take().or(self.display_name);
        let signature = self.account.signature.take().unwrap_or(self.signature);
        let client = EmailClient::new(self.account)?;
        Ok(AccountSession {
            account_name: self.account_name,
            from,
            from_name,
            signature,
            account_names: self.account_names,
            client,
        })
    }
}

/// Parses the config at `paths` and opens a session for `account`
/// (the default account when `None`).
pub fn open_account_session(paths: &[PathBuf], account: Option<&str>) -> Result<AccountSession> {
    let mut config = load_config(paths)?;
    let account_names = sorted_account_names(&config);
    let display_name = config.display_name.take();
    let signature = config.signature.take().unwrap_or_default();
    let (account_name, account) = config
        .take_account(account)?
        .with_context(|| missing_account(account))?;
    SessionSource {
        account_name,
        account,
        display_name,
        signature,
        account_names,
    }
    .open()
}

/// The switcher's config re-read: just the account names at `paths`.
pub fn config_account_names(paths: &[PathBuf]) -> Result<Vec<String>> {
    Ok(sorted_account_names(&load_config(paths)?))
}

/// Account names in stable (sorted) order; the map iteration order
/// behind them is random per process.
pub fn sorted_account_names(config: &Config) -> Vec<String> {
    let mut names: Vec<String> = config.accounts.keys().cloned().collect();
    names.sort();
    names
}

fn load_config(paths: &[PathBuf]) -> Result<Config> {
    Config::from_paths_or_default(paths)?.context("No configuration file found")
}

fn missing_account(account: Option<&str>) -> String {
    match account {
        Some(name) => format!("Account `{name}` not found in the configuration"),
        None => String::from("No default account in the configuration"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_account() -> AccountConfig {
        AccountConfig {
            default: false,
            imap: None,
            smtp: None,
            jmap: None,
            maildir: None,
            m2dir: None,
            from: None,
            from_name: None,
            signature: None,
            signature_delim: None,
            downloads_dir: None,
        }
    }

    #[test]
    fn account_names_are_sorted() {
        let mut config = Config::default();
        config.accounts.insert("zulu".into(), stub_account());
        config.accounts.insert("alpha".into(), stub_account());

        assert_eq!(sorted_account_names(&config), ["alpha", "zulu"]);
    }

    #[test]
    fn missing_config_file_is_reported() {
        let missing = std::env::temp_dir().join("himalaya-tui-no-such-config.toml");

        assert!(config_account_names(&[missing]).is_err());
    }
}
