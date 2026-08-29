//! # Account configuration
//!
//! What becomes of the account [`super::discover`] built: a file to
//! create, a block to append to the one already there, or nothing.
//!
//! This is the himalaya CLI's `configure` command minus its printing.
//! The offer is kept so that stdout is the whole of the difference
//! between the two wizards: declining prints nothing, and the account is
//! opened either way, for this session alone when nothing was written.
//!
//! Appending is a plain text append rather than a re-serialization, so
//! comments, ordering and hand-written formatting come out untouched.

use std::{
    fs::{self, OpenOptions},
    io::{IsTerminal, Write, stdin},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use pimalaya_cli::prompt;
use pimalaya_config::toml::TomlConfig;

use crate::{
    config::{AccountConfig, Config},
    wizard::discover::{self, CONFIG_SAMPLE_URL},
};

/// Runs the wizard and hands back the account to open, with its name.
///
/// `seed` answers the first prompt outright, which is what the
/// positional argument is for. The account is offered to the
/// configuration file on the way out, and comes back either way.
pub fn run(seed: Option<&str>, config_paths: &[PathBuf]) -> Result<(String, AccountConfig)> {
    if !stdin().is_terminal() {
        bail!(
            "Configuring needs a terminal to prompt on, \
             write the configuration by hand instead: {CONFIG_SAMPLE_URL}"
        );
    }

    let path = Config::target_path(config_paths)?;
    let existing = ExistingConfig::read(&path)?;

    let (base_name, mut account) = discover::run(seed)?;
    let name = account_name(&base_name, existing.as_ref());

    // NOTE: a second `default = true` would make the account the CLI
    // picks depend on map ordering.
    account.default = !existing.as_ref().is_some_and(|config| config.has_default);

    offer_to_save(&path, existing.is_some(), &name, &account)?;

    Ok((name, account))
}

/// Introduces himalaya-tui and names the configuration file missing at
/// `path`.
///
/// Printed before the offer a bare `himalaya-tui` raises when it finds
/// no configuration, so the wizard introduces itself to someone who did
/// not ask for it. A run that asked for it skips this.
pub fn print_welcome(path: &Path) {
    eprintln!();
    eprintln!("Welcome to Himalaya TUI, the TUI to manage emails.");
    eprintln!();
    eprintln!("Himalaya TUI talks to your existing mailbox over IMAP, JMAP or a local");
    eprintln!("Maildir. It needs one account to know which mailbox to read, and no");
    eprintln!("configuration file was found at:");
    eprintln!();
    eprintln!("  {}", path.display());
    eprintln!();
    eprintln!("The wizard discovers a provider's settings from your email address, tests");
    eprintln!("the connection and generates a ready-to-use account. Everything discovery");
    eprintln!("does not cover is written by hand, and every field is documented at:");
    eprintln!();
    eprintln!("  {CONFIG_SAMPLE_URL}");
    eprintln!();
    eprintln!("At anytime, you can open a new account with the command:");
    eprintln!();
    eprintln!("  himalaya-tui --no-config");
    eprintln!();
}

/// What a configuration already on disk constrains in a new account.
///
/// Namely the names it cannot take, and whether one of its accounts
/// already claims the default.
struct ExistingConfig {
    names: Vec<String>,
    has_default: bool,
}

impl ExistingConfig {
    /// Reads the configuration at `path`, or `None` when no file is
    /// there.
    ///
    /// A parse failure is an error rather than a `None`: appending to a
    /// broken document would bury the actual problem under a second one.
    fn read(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let config = Config::from_paths(&[path.to_path_buf()])
            .with_context(|| format!("Read the configuration at {}", path.display()))?;

        Ok(Some(Self {
            names: config.accounts.keys().cloned().collect(),
            has_default: config.accounts.values().any(|account| account.default),
        }))
    }
}

/// The name discovery proposes, suffixed until it is free.
///
/// Not prompted, the name being only the TOML table key. It still has
/// to be free: a second `[accounts.<name>]` table makes the whole
/// document fail to parse, taking the working accounts down with it.
fn account_name(base: &str, existing: Option<&ExistingConfig>) -> String {
    let taken = existing
        .map(|config| config.names.as_slice())
        .unwrap_or(&[]);

    if !taken.iter().any(|name| name == base) {
        return base.to_string();
    }

    let mut suffix = 2;

    loop {
        let name = format!("{base}-{suffix}");

        if !taken.contains(&name) {
            return name;
        }

        suffix += 1;
    }
}

/// Offers to write the generated account to the configuration file.
///
/// Creates the file or appends to the one already there. Declining
/// writes nothing and says nothing: the account is opened all the same.
fn offer_to_save(path: &Path, exists: bool, name: &str, account: &AccountConfig) -> Result<()> {
    let prompt = if exists {
        format!("Append account `{name}` to {}?", path.display())
    } else {
        format!("Save this account to {}?", path.display())
    };

    if !prompt::bool(prompt, true)? {
        return Ok(());
    }

    let document = account.render(name)?;

    if exists {
        append(path, &document)?;
    } else {
        save(path, &document)?;
    }

    print_saved(path, name, account.default);

    Ok(())
}

/// Writes the account as a new configuration file, creating its
/// directory.
fn save(path: &Path, document: &str) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Create the config directory {}", parent.display()))?;
    }

    fs::write(path, document).with_context(|| format!("Write the config file {}", path.display()))
}

/// Appends the account to the configuration file already there.
fn append(path: &Path, document: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .with_context(|| format!("Open the config file {}", path.display()))?;

    // NOTE: the leading newline separates the two tables, and terminates
    // the last line when the file ends without one.
    write!(file, "\n{}", document.trim_end())
        .with_context(|| format!("Append to the config file {}", path.display()))?;

    writeln!(file).with_context(|| format!("Append to the config file {}", path.display()))
}

/// Tells where the account landed and under which name.
///
/// The name matters here because it was never asked for: an account
/// that did not claim the default is only reachable through `-a`.
fn print_saved(path: &Path, name: &str, default: bool) {
    eprintln!();
    eprintln!("Account `{name}` saved to {}.", path.display());

    if !default {
        eprintln!("Another account holds the default, so name this one with `-a {name}`.");
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    static NEXT_CONFIG: AtomicUsize = AtomicUsize::new(0);

    /// A path in the temporary directory no other test writes to.
    fn config_path() -> PathBuf {
        let id = NEXT_CONFIG.fetch_add(1, Ordering::Relaxed);
        env::temp_dir().join(format!("himalaya-tui-configure-{id}.toml"))
    }

    /// A minimal account naming a Maildir root, the backend needing no
    /// network.
    fn account(default: bool) -> AccountConfig {
        AccountConfig {
            default,
            maildir: Some(crate::config::MaildirConfig {
                root: PathBuf::from("/tmp/mail"),
            }),
            ..Default::default()
        }
    }

    #[test]
    fn a_generated_account_parses_back() {
        let document = account(true).render("perso").expect("render the account");
        let config: Config = toml::from_str(&document).expect("parse the generated config");
        let account = &config.accounts["perso"];

        assert_eq!(config.accounts.len(), 1);
        assert!(account.default);
        assert_eq!(
            account.maildir.as_ref().map(|c| c.root.as_path()),
            Some(Path::new("/tmp/mail"))
        );

        // A generated document holds what was configured, every other
        // field staying at its default.
        assert!(!document.contains("imap"));
        assert!(!document.contains("signature"));

        // The account name heads the block, and `default` reads before
        // the backend it qualifies.
        let lines: Vec<&str> = document.lines().collect();
        assert_eq!(lines[0], "[accounts.perso]");
        assert_eq!(lines[1], "default = true");
    }

    #[test]
    fn the_endpoint_reads_before_its_credentials() {
        // Serialized alphabetically, `maildir.root` would sit under any
        // sibling sorting before it: the renderer lifts a group's
        // endpoint to its top.
        let document = account(true).render("perso").expect("render the account");
        let maildir: Vec<&str> = document
            .lines()
            .filter(|line| line.starts_with("maildir."))
            .collect();

        assert_eq!(maildir, ["maildir.root = \"/tmp/mail\""]);
    }

    #[test]
    fn an_appended_account_keeps_the_existing_one() {
        let path = config_path();

        // No trailing newline, the shape an appended block has to
        // survive without merging into the last line.
        fs::write(
            &path,
            "# my accounts\n[accounts.work]\ndefault = true\nmaildir.root = \"/tmp/work\"",
        )
        .expect("write the existing config");

        let existing = ExistingConfig::read(&path)
            .expect("read the existing config")
            .expect("an existing config");

        assert_eq!(existing.names, ["work"]);
        assert!(existing.has_default);

        let document = account(!existing.has_default)
            .render("perso")
            .expect("render the account");
        append(&path, &document).expect("append the generated account");

        let content = fs::read_to_string(&path).expect("read back");
        let config: Config = toml::from_str(&content).expect("parse the appended config");

        assert_eq!(config.accounts.len(), 2);

        // Exactly one default, and the comment is still there.
        let defaults = config
            .accounts
            .values()
            .filter(|account| account.default)
            .count();
        assert_eq!(defaults, 1);
        assert!(config.accounts["work"].default);
        assert!(content.starts_with("# my accounts"));

        fs::remove_file(&path).expect("remove the config");
    }

    #[test]
    fn a_taken_name_gets_a_suffix() {
        let existing = ExistingConfig {
            names: vec!["perso".to_string(), "perso-2".to_string()],
            has_default: true,
        };

        assert_eq!(account_name("perso", None), "perso");
        assert_eq!(account_name("perso", Some(&existing)), "perso-3");
        assert_eq!(account_name("work", Some(&existing)), "work");
    }

    #[test]
    fn a_missing_configuration_constrains_nothing() {
        let existing = ExistingConfig::read(&config_path()).expect("read a missing config");

        assert!(existing.is_none());
    }
}
