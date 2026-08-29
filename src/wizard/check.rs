//! # Connection tests
//!
//! Validates a discovered account before the session opens, so a bad
//! credential or endpoint stops the wizard where it was typed rather
//! than surfacing as a blank interface.
//!
//! Each test is the ordinary client constructor: opening a session is
//! what the TUI does next anyway, and a second code path could pass
//! where the real one fails.

use anyhow::Result;
#[cfg(feature = "imap")]
use io_sasl::mechanism::SaslMechanism;

use crate::config::AccountConfig;
#[cfg(feature = "imap")]
use crate::config::ImapConfig;

/// Tests every backend the account configured, failing on the first
/// error.
///
/// For the flows that configure without connecting (the local
/// backends): the IMAP+SMTP and JMAP flows test each connection as they
/// prompt for it, and tell the wizard to skip this.
pub fn test_account(account_config: &AccountConfig) -> Result<()> {
    #[cfg(feature = "imap")]
    if let Some(config) = &account_config.imap {
        connect_imap(config)?;
    }

    #[cfg(feature = "jmap")]
    if let Some(config) = &account_config.jmap {
        crate::jmap::client::JmapClient::new(config.clone())?;
    }

    #[cfg(feature = "maildir")]
    if let Some(config) = &account_config.maildir {
        check_root("Maildir", &config.root)?;
    }

    #[cfg(feature = "smtp")]
    if let Some(config) = &account_config.smtp {
        connect_smtp(config)?;
    }

    Ok(())
}

/// Opens an IMAP session and drops it.
#[cfg(feature = "imap")]
pub fn connect_imap(config: &ImapConfig) -> Result<()> {
    crate::imap::client::ImapClient::new(config.clone())?;
    Ok(())
}

/// Opens an SMTP session and drops it.
#[cfg(feature = "smtp")]
pub fn connect_smtp(config: &crate::config::SmtpConfig) -> Result<()> {
    crate::smtp::client::SmtpClient::new(config.clone())?;
    Ok(())
}

/// The SASL mechanisms `server` advertises, most preferred first.
///
/// The connection is opened unauthenticated (implicit TLS or STARTTLS)
/// only to read CAPABILITY, so the wizard offers just what the server
/// supports; LOGIN is ordered last.
#[cfg(feature = "imap")]
pub fn probe_imap_mechanisms(server: &str, starttls: bool) -> Result<Vec<SaslMechanism>> {
    use io_imap::{
        client::{ImapClientStd, default_alpn},
        rfc3501::capability::available_auth_mechanisms,
        session::ImapSessionOpenOptions,
    };
    use io_sasl::mechanism::Sasl;

    use crate::{config::TlsConfig, imap::client::parse_imap_server};

    let tls = TlsConfig::default().into_tls(default_alpn());
    let server = parse_imap_server(server)?;
    let opts = ImapSessionOpenOptions {
        starttls,
        ..Default::default()
    };
    let (_client, capabilities) = ImapClientStd::connect(&server, &tls, None::<Sasl>, opts)?;

    Ok(available_auth_mechanisms(&capabilities))
}

/// Fails when a local backend's root is missing or is not a directory.
///
/// The wizard never creates it: a typo would otherwise silently open an
/// empty mailbox.
#[cfg(feature = "maildir")]
fn check_root(label: &str, root: &std::path::Path) -> Result<()> {
    use anyhow::bail;

    if !root.is_dir() {
        bail!(
            "{label} root `{}` does not exist or is not a directory",
            root.display()
        );
    }

    Ok(())
}
