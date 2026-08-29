//! # Account discovery
//!
//! The half of the wizard that decides what the account is. What becomes
//! of it belongs to [`super::configure`].
//!
//! One prompt takes an email address, a server URL, or a local folder
//! path: an email or bare domain searches every reachable service (see
//! [`super::search`]), a URL narrows that search to its scheme, and a
//! folder is a local Maildir.
//!
//! Only what discovery covers is configured: an input it finds nothing
//! for stops the wizard and points at the documented sample. No OAuth
//! 2.0 grant runs here either, a grant only unlocking the token brokers
//! behind the API token prompt (see [`super::secret`]).

use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result, bail};
#[cfg(all(feature = "imap", feature = "smtp"))]
use io_pim_discovery::compose::config::DiscoverySecurity;
use pimalaya_cli::{prompt, spinner::Spinner};
use url::Url;

#[cfg(feature = "jmap")]
use crate::config::JmapConfig;
#[cfg(feature = "maildir")]
use crate::config::MaildirConfig;
#[cfg(all(feature = "imap", feature = "smtp"))]
use crate::config::{ImapConfig, SmtpConfig};
#[cfg(all(feature = "imap", feature = "smtp"))]
use crate::wizard::imap_smtp;
#[cfg(feature = "jmap")]
use crate::wizard::jmap;
use crate::{
    config::AccountConfig,
    wizard::{
        check,
        search::{self, Discovered, DiscoveredKind},
    },
};

/// The documented sample configuration.
///
/// Shown in the welcome banner, and pointed at when discovery finds
/// nothing to configure automatically.
pub const CONFIG_SAMPLE_URL: &str =
    "https://github.com/pimalaya/himalaya-tui/blob/master/config.sample.toml";

/// The backend config a flow produced, folded into an [`AccountConfig`].
enum Chosen {
    #[cfg(all(feature = "imap", feature = "smtp"))]
    ImapSmtp(Box<ImapConfig>, Option<Box<SmtpConfig>>),
    #[cfg(feature = "jmap")]
    Jmap(Box<JmapConfig>),
    #[cfg(feature = "maildir")]
    Maildir(MaildirConfig),
}

/// Discovers one account from a single prompt, tests it, and names it.
///
/// What happens to that account belongs to [`super::configure`]: this
/// is the discovery half. `seed` answers the prompt on the caller's
/// behalf, for the positional argument naming a throwaway account.
pub fn run(seed: Option<&str>) -> Result<(String, AccountConfig)> {
    let prompted;
    let input = match seed {
        Some(seed) => seed,
        None => {
            prompted = prompt::text::<&str>("Email:", None)?;
            prompted.as_str()
        }
    };
    let input = input.trim();
    if input.is_empty() {
        bail!("Empty input: enter an email address, a server URL, or a folder path");
    }

    // NOTE: the name is just the TOML table key, so it is derived rather
    // than prompted; the user renames it by hand.
    let account_name = default_account_name(input);
    let (account, tested) = build_account(&account_name, input)?;

    // A bad credential or endpoint fails here rather than dropping into
    // an interface that can show nothing. The IMAP+SMTP and JMAP flows
    // already tested each connection as they prompted for it.
    if !tested {
        let spinner = Spinner::start("Testing account configuration");
        if let Err(err) = check::test_account(&account) {
            spinner.failure("Account configuration test failed");
            return Err(err);
        }
        spinner.success("Account configuration is valid");
    }

    Ok((account_name, account))
}

/// The result of a configure flow.
///
/// `tested` reports a flow that already validated its connections, so
/// the caller skips the final account test; `aliases` holds the
/// `mailbox.alias.*` entries discovered from the server.
struct Outcome {
    chosen: Chosen,
    tested: bool,
    aliases: HashMap<String, String>,
}

impl Outcome {
    /// An untested outcome with no aliases, for the local backends.
    ///
    /// Their validation is deferred to the final account test.
    fn untested(chosen: Chosen) -> Self {
        Self {
            chosen,
            tested: false,
            aliases: HashMap::new(),
        }
    }
}

/// Orients the setup from the input shape into an [`AccountConfig`].
///
/// The flag reports a flow that already validated its connections. The
/// account is left non-default, since claiming the default depends on
/// what the configuration holds and only [`super::configure`] reads it.
fn build_account(account_name: &str, input: &str) -> Result<(AccountConfig, bool)> {
    let Outcome {
        chosen,
        tested,
        aliases,
    } = if is_path(input) {
        Outcome::untested(configure_local(input)?)
    } else {
        configure_discovery(account_name, input)?
    };

    let mut account = AccountConfig {
        default: false,
        ..Default::default()
    };

    match chosen {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        Chosen::ImapSmtp(imap, smtp) => {
            account.imap = Some(*imap);
            account.smtp = smtp.map(|smtp| *smtp);
        }
        #[cfg(feature = "jmap")]
        Chosen::Jmap(jmap) => account.jmap = Some(*jmap),
        #[cfg(feature = "maildir")]
        Chosen::Maildir(maildir) => account.maildir = Some(maildir),
    }

    // NOTE: the aliases are the himalaya CLI's way of addressing a
    // mailbox; the TUI resolves names live and reads none of them, but
    // one file backs both binaries.
    account.mailbox.aliases = aliases;

    account.from = prompted_email(input).map(ToString::to_string);

    Ok((account, tested))
}

/// Runs the discovery flow for an email, a bare domain or a server URL.
///
/// The services reachable from it are searched, kept only when this
/// build and the URL scheme support them, then picked from; nothing
/// discovered stops the wizard (see [`stop_undiscovered`]).
fn configure_discovery(account_name: &str, input: &str) -> Result<Outcome> {
    // A URL discovers from its host with its scheme narrowing the
    // results; an email or bare domain discovers from the domain.
    let (email, scheme) = if input.contains("://") {
        let url = Url::parse(input).with_context(|| format!("Invalid server URL `{input}`"))?;
        let host = url.host_str().unwrap_or_default().to_string();
        (format!("@{host}"), Some(url.scheme().to_string()))
    } else if input.contains('@') {
        (input.to_string(), None)
    } else {
        (format!("@{input}"), None)
    };

    let spinner = Spinner::start("Searching for server settings");
    let mut found = search::search(&email)?;
    retain_supported(&mut found);
    if let Some(scheme) = &scheme {
        retain_scheme(&mut found, scheme)?;
    }

    if found.is_empty() {
        spinner.failure("No configuration found");
        return stop_undiscovered(input);
    }
    spinner.success(format!("Found {} configuration(s)", found.len()));

    let default = found.first().cloned();
    let choice = prompt::item("Choose a configuration:", found, default)?;

    dispatch(account_name, &email, choice)
}

/// Keeps only the discovered entries `scheme` asked for.
///
/// `imap` and `imaps` keep IMAP + SMTP, `imaps` requiring implicit TLS,
/// and the HTTP-family schemes keep JMAP. A proprietary entry is
/// dropped: the user named an open protocol.
fn retain_scheme(found: &mut Vec<Discovered>, scheme: &str) -> Result<()> {
    match scheme {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        "imap" | "imaps" => {
            let tls_only = scheme == "imaps";
            found.retain(|entry| match &entry.kind {
                DiscoveredKind::ImapSmtp { imap, .. } => {
                    !tls_only || imap.security == DiscoverySecurity::Tls
                }
                _ => false,
            });
        }
        "jmap" | "jmaps" | "http" | "https" => {
            found.retain(|entry| matches!(entry.kind, DiscoveredKind::Jmap(_)));
        }
        other => bail!("Unsupported server scheme `{other}`"),
    }

    Ok(())
}

/// Stops the wizard when discovery found nothing for `input`.
///
/// It points at the documented sample rather than dropping into a
/// hand-entry flow: the wizard only ever configures what it can
/// discover automatically.
fn stop_undiscovered(input: &str) -> Result<Outcome> {
    bail!(
        "Could not automatically discover a configuration for `{input}`.\n\n\
         Write your account configuration by hand instead, starting from the \
         documented sample:\n  {CONFIG_SAMPLE_URL}"
    )
}

/// Configures the backend behind a discovered entry.
///
/// The IMAP+SMTP and JMAP flows test their connections inline, marking
/// the outcome tested, and discover their `mailbox.alias.*` on the same
/// session.
#[cfg_attr(
    all(feature = "imap", feature = "smtp", feature = "jmap"),
    allow(unreachable_patterns)
)]
fn dispatch(account_name: &str, email: &str, choice: Discovered) -> Result<Outcome> {
    match &choice.kind {
        #[cfg(all(feature = "imap", feature = "smtp"))]
        DiscoveredKind::ImapSmtp { .. } => {
            let (imap, smtp, aliases) =
                imap_smtp::configure_discovered(account_name, email, &choice)?;
            Ok(Outcome {
                chosen: Chosen::ImapSmtp(Box::new(imap), smtp.map(Box::new)),
                tested: true,
                aliases,
            })
        }
        #[cfg(feature = "jmap")]
        DiscoveredKind::Jmap(_) => {
            let (jmap, aliases) = jmap::configure_discovered(account_name, email, &choice)?;
            Ok(Outcome {
                chosen: Chosen::Jmap(Box::new(jmap)),
                tested: true,
                aliases,
            })
        }
        kind => bail!("Configuration `{kind:?}` is not supported by this build"),
    }
}

/// Configures the Maildir a typed folder path points at.
#[cfg(feature = "maildir")]
fn configure_local(input: &str) -> Result<Chosen> {
    let raw = input.strip_prefix("file://").unwrap_or(input);
    let root = shellexpand::tilde(raw).into_owned();
    if !Path::new(&root).is_dir() {
        bail!("No such folder `{raw}`");
    }

    Ok(Chosen::Maildir(MaildirConfig { root: root.into() }))
}

/// Rejects a folder path when no local backend is compiled in.
#[cfg(not(feature = "maildir"))]
fn configure_local(input: &str) -> Result<Chosen> {
    bail!("`{input}` looks like a folder path, but no local backend is compiled in")
}

/// Drops the discovered entries whose backend is not compiled in.
fn retain_supported(found: &mut Vec<Discovered>) {
    found.retain(|entry| match entry.kind {
        DiscoveredKind::ImapSmtp { .. } => cfg!(all(feature = "imap", feature = "smtp")),
        DiscoveredKind::Jmap(_) => cfg!(feature = "jmap"),
        // NOTE: this binary reaches a Google or Microsoft account over
        // IMAP + SMTP, the proprietary APIs being the CLI's.
        DiscoveredKind::Gmail | DiscoveredKind::Msgraph => false,
    });
}

/// The default account name proposed from the input shape.
///
/// The first label of its domain, or the folder name of a local path.
fn default_account_name(input: &str) -> String {
    if is_path(input) {
        let raw = input.strip_prefix("file://").unwrap_or(input);
        return Path::new(raw)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("personal")
            .to_string();
    }

    if let Ok(url) = Url::parse(input)
        && let Some(host) = url.host_str()
    {
        return first_label(host);
    }

    match input.rsplit_once('@') {
        Some((_, domain)) => first_label(domain),
        None => first_label(input),
    }
}

/// The input read back as an email address, or [`None`] when it is not
/// one.
///
/// A server URL may carry a userinfo part and so hold an `@` of its
/// own, which is a credential and not an address, hence the check for a
/// scheme before the one for a local part.
fn prompted_email(input: &str) -> Option<&str> {
    if is_path(input) || input.contains("://") {
        return None;
    }

    let (local, domain) = input.rsplit_once('@')?;

    if local.is_empty() || domain.is_empty() {
        return None;
    }

    Some(input)
}

/// The first dot-separated label of a host or domain.
fn first_label(host: &str) -> String {
    host.split('.').next().unwrap_or(host).to_string()
}

/// Whether the input names a filesystem path, not a network endpoint.
fn is_path(input: &str) -> bool {
    input.starts_with("file://")
        || input.starts_with('/')
        || input.starts_with('~')
        || input.starts_with("./")
        || input.starts_with("../")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_name_defaults_to_the_first_domain_label() {
        // Email: the domain's first label, never the local part.
        assert_eq!(default_account_name("clement.douin@posteo.net"), "posteo");
        assert_eq!(default_account_name("alice@mail.example.co.uk"), "mail");
        // Bare domain (as discovery synthesizes it) and plain domain.
        assert_eq!(default_account_name("@posteo.net"), "posteo");
        assert_eq!(default_account_name("posteo.net"), "posteo");
    }

    #[test]
    fn account_name_defaults_to_the_last_path_component() {
        assert_eq!(
            default_account_name("/home/alice/mail/personal"),
            "personal"
        );
        assert_eq!(default_account_name("~/mail/work"), "work");
        assert_eq!(default_account_name("file:///var/mail/archive"), "archive");
    }

    #[test]
    fn only_an_address_is_kept_as_the_account_from() {
        assert_eq!(
            prompted_email("alice@example.org"),
            Some("alice@example.org")
        );

        // A bare domain, as discovery also synthesizes it, names no
        // mailbox; neither does a folder or a server URL, whose `@`
        // would be a credential.
        assert_eq!(prompted_email("@example.org"), None);
        assert_eq!(prompted_email("example.org"), None);
        assert_eq!(prompted_email("~/mail/work"), None);
        assert_eq!(prompted_email("imaps://alice@imap.example.org"), None);
    }

    #[test]
    fn discovered_aliases_render_as_a_mailbox_alias_table() {
        let mut account = AccountConfig {
            from: Some("me@posteo.net".to_string()),
            ..Default::default()
        };
        account
            .mailbox
            .aliases
            .insert("inbox".to_string(), "INBOX".to_string());

        let rendered = account.render("posteo").expect("render the account");

        assert!(rendered.contains("[accounts.posteo]"));
        assert!(rendered.contains("mailbox.alias.inbox = \"INBOX\""));

        // The account is written under the CLI's spelling, and the
        // identity says what the account is, so it reads before the
        // mailboxes it names.
        let email = rendered.find("email = ").expect("the address is rendered");
        let alias = rendered
            .find("mailbox.alias")
            .expect("the alias is rendered");
        assert!(email < alias);
    }
}
