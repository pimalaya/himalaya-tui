//! # Secret prompts
//!
//! The prompts shared by the discovered-backend wizards (IMAP, SMTP,
//! JMAP), delegating to pimalaya-cli's OS-aware pickers.
//!
//! [`configure_password`] offers the OS keyrings, [`configure_token`]
//! the OAuth 2.0 token brokers (Ortie, pizauth, oama), and both a custom
//! command or a raw value. himalaya-tui only reads a secret: the value
//! must already be stored, and a missing one surfaces on the next test.

use std::process::Command;

use anyhow::{Result, bail};
use pimalaya_cli::wizard::keyring::{self, SecretChoice};
use pimalaya_config::{command::shell, secret::Secret};

/// Prompts for a password [`Secret`] through the shared keyring picker.
///
/// `key_default` seeds the keyring entry, typically
/// `<account>-<protocol>`. The entry is used verbatim, so a pre-existing
/// secret is read exactly as named.
pub fn configure_password(label: &str, key_default: &str) -> Result<Secret> {
    to_secret(keyring::prompt_secret(label, key_default)?)
}

/// Prompts for an API token [`Secret`] through the shared token picker.
///
/// It combines the OS keyrings with the OAuth 2.0 brokers when `oauth`
/// is true, a broker printing a fresh token on every read.
/// `key_default` seeds the keyring entry or the broker account handle.
pub fn configure_token(label: &str, key_default: &str, oauth: bool) -> Result<Secret> {
    to_secret(keyring::prompt_token(label, key_default, oauth)?)
}

/// The [`Secret`] a picker choice stands for.
fn to_secret(choice: SecretChoice) -> Result<Secret> {
    Ok(match choice {
        SecretChoice::Command(argv) => command_secret(argv)?,
        SecretChoice::Shell(line) => shell_secret(&line)?,
        SecretChoice::Raw(secret) => Secret::Raw(secret),
    })
}

/// Builds a [`Secret::Command`] from an argv, as a known keyring
/// provider or token broker yields.
///
/// No shell is involved, and it serializes back as a TOML array.
fn command_secret(argv: Vec<String>) -> Result<Secret> {
    let Some((program, args)) = argv.split_first() else {
        bail!("Empty command for secret");
    };

    let mut cmd = Command::new(program);
    cmd.args(args);
    Ok(Secret::Command(cmd))
}

/// Builds a [`Secret::Command`] from a hand-typed shell command line.
///
/// It serializes back as a TOML string, unlike an argv.
fn shell_secret(line: &str) -> Result<Secret> {
    let line = line.trim();
    if line.is_empty() {
        bail!("Empty shell command for secret");
    }

    Ok(Secret::Command(shell(line)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_command_secret_is_rejected() {
        assert!(command_secret(Vec::new()).is_err());
    }

    #[test]
    fn blank_shell_secret_is_rejected() {
        assert!(shell_secret("   ").is_err());
    }
}
