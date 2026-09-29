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
//! Two rules guard it, and they are the CLI's: the account name has to
//! be free, two tables of one name making the whole document fail to
//! parse, and the new account claims the default only when no other one
//! does. Both are properties of the accounts table the two binaries
//! share, so the block written here is one himalaya reads unchanged.

use std::{
    fmt,
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
/// The CLI's `ConfigureCommand::execute` step for step, save for two
/// deviations the interface needs. It is a function rather than a
/// subcommand, there being no `configure` to type, and it returns the
/// account instead of ending on a document, so the session opens on
/// what was just discovered.
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

    // NOTE: a second `default = true` would make the account both
    // binaries pick depend on map ordering, so the generated one claims
    // the default only when no other account does.
    let default = !existing.as_ref().is_some_and(|config| config.has_default);
    account.default = default;

    let generated = GeneratedConfig {
        document: account.render(&name)?,
        name,
        default,
    };

    // NOTE: no stdout branch here, unlike the CLI's: the interface takes
    // the terminal over the moment this returns, so there is no document
    // to redirect and nothing that would read it.
    match existing {
        Some(_) => append_or_skip(&path, &generated)?,
        None => save_or_skip(&path, &generated)?,
    }

    Ok((generated.name, account))
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

/// The generated account, as the offer to file it takes it.
///
/// The CLI's `ConfigureOutput` without its `Serialize` and `JsonSchema`
/// derives, and named for what it is here: nothing prints this
/// document, it only ever reaches a file.
struct GeneratedConfig {
    /// The account name, which is the `[accounts.<name>]` table key.
    name: String,
    /// Whether the account claims the default.
    default: bool,
    /// The rendered TOML document.
    document: String,
}

impl fmt::Display for GeneratedConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // NOTE: the trailing newline terminates the document, whichever
        // shape the renderer left it in.
        writeln!(f, "{}", self.document.trim_end())
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

/// Offers to write the generated account to a configuration file that
/// does not exist yet, writing nothing when the offer is declined.
///
/// The CLI's `save_or_print` minus the print: a declined offer leaves
/// the account unfiled rather than on stdout, the interface opening on
/// it for this session alone.
fn save_or_skip(path: &Path, config: &GeneratedConfig) -> Result<()> {
    let prompt = format!("Save this account to {}?", path.display());

    if !prompt::bool(prompt, true)? {
        return Ok(());
    }

    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("Create the config directory {}", parent.display()))?;
    }

    fs::write(path, config.to_string())
        .with_context(|| format!("Write the config file {}", path.display()))?;

    print_saved(path, config);

    Ok(())
}

/// Offers to append the generated account to the configuration file
/// already there, writing nothing when the offer is declined.
///
/// The CLI's `append_or_print` minus the print, as [`save_or_skip`] is.
fn append_or_skip(path: &Path, config: &GeneratedConfig) -> Result<()> {
    let prompt = format!("Append account `{}` to {}?", config.name, path.display());

    if !prompt::bool(prompt, true)? {
        return Ok(());
    }

    let mut file = OpenOptions::new()
        .append(true)
        .open(path)
        .with_context(|| format!("Open the config file {}", path.display()))?;

    // NOTE: appending text keeps every comment and hand-written line as
    // they are, which re-serializing the document would not. The leading
    // newline separates the two tables, and terminates the last line of a
    // file that ends without one.
    write!(file, "\n{config}")
        .with_context(|| format!("Append to the config file {}", path.display()))?;

    print_saved(path, config);

    Ok(())
}

/// Tells where the account landed and under which name.
///
/// The name matters here because it was never asked for: an account
/// that did not claim the default is only reachable through `-a`.
///
/// Nothing to run closes it, unlike the CLI's: the interface opens on
/// this account the moment the wizard returns.
fn print_saved(path: &Path, config: &GeneratedConfig) {
    let name = &config.name;

    eprintln!();
    eprintln!("Account `{name}` saved to {}.", path.display());

    if !config.default {
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
        let mut file = OpenOptions::new().append(true).open(&path).expect("open");
        write!(file, "\n{document}").expect("append the generated account");
        drop(file);

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
