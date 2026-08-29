//! # IMAP client
//!
//! Himalaya TUI wrapper around [`io_imap::client::ImapClientStd`].
//!
//! The session is opened once via [`ImapClient::new`], then the sibling
//! backend module's adapter methods reach the inner client through the
//! [`Deref`]/[`DerefMut`] passthrough.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use io_imap::{
    client::{ImapClient as _, ImapClientStd as Inner, default_alpn, default_port},
    session::ImapSessionOpenOptions,
    types::{
        IntoStatic,
        core::{IString, NString},
    },
};
use io_sasl::mechanism::Sasl;
use url::Url;

use crate::config::{ImapConfig, ImapIdConfig, parse_server};

/// Live IMAP client wrapping the io-imap session.
///
/// State is deliberately minimal: the shared-API methods re-SELECT
/// before every operation and never consult cached capabilities, so
/// nothing beyond the inner client needs keeping.
pub struct ImapClient {
    inner: Inner,
}

impl ImapClient {
    /// Opens the IMAP connection: TCP/TLS/STARTTLS, greeting then SASL.
    pub fn new(config: ImapConfig) -> Result<Self> {
        let tls = config
            .tls
            .into_tls(config.alpn.unwrap_or_else(default_alpn));
        let server = parse_imap_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // NOTE: a unix:// sirup socket presents an already
            // authenticated session (PREAUTH greeting), so no SASL is
            // negotiated.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server
                    .host_str()
                    .ok_or_else(|| anyhow!("Cannot derive host from IMAP server `{server}`"))?;
                // NOTE: url does not know the imap(s) default ports, so
                // fall back to the scheme defaults io-imap connects with.
                let port = server.port().unwrap_or(default_port(server.scheme()));
                Some(cfg.try_into_sasl(host, port)?)
            }
            None => None,
        };
        let opts = ImapSessionOpenOptions {
            starttls: config.starttls,
            auto_id: resolve_auto_id_params(&config.id)?,
            sasl_ir: config.sasl_ir,
        };

        let (inner, _capabilities) = Inner::connect(&server, &tls, sasl, opts)?;

        Ok(Self { inner })
    }

    /// Checks liveness with an IMAP `NOOP`, which also polls for any
    /// pending untagged update.
    pub fn ping(&mut self) -> Result<()> {
        self.inner.noop()?;
        Ok(())
    }
}

impl Deref for ImapClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ImapClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Parses an IMAP server string into a URL.
///
/// Accepts an `imap://` or `imaps://` URL, a bare authority defaulting
/// to the secure `imaps://`, or a `unix://` socket path for a local
/// proxy such as sirup. Any other scheme is rejected.
pub fn parse_imap_server(server: &str) -> Result<Url> {
    parse_server(server, "imaps", &["imap", "imaps", "unix"])
}

/// Resolves an [`ImapIdConfig`] into the wire-level `ID` parameters.
///
/// [`None`] when `auto = false`, otherwise each key maps to the canned
/// value when the user set `true` and the key is well-known, and to
/// `NIL` otherwise, an unknown `true` key also logging a warning.
pub fn resolve_auto_id_params(
    config: &ImapIdConfig,
) -> Result<Option<Vec<(IString<'static>, NString<'static>)>>> {
    if !config.auto {
        return Ok(None);
    }

    let mut params = Vec::with_capacity(config.fields.len());

    for (key, &use_canned) in &config.fields {
        let ikey = IString::try_from(key.clone())
            .map_err(|err| anyhow!("Invalid IMAP ID parameter key `{key}`: {err}"))?
            .into_static();

        let nval = if use_canned {
            match canned_imap_id_value(key) {
                Some(value) => NString::try_from(value)
                    .map_err(|err| {
                        anyhow!("Invalid canned IMAP ID value `{value}` for `{key}`: {err}")
                    })?
                    .into_static(),
                None => {
                    log::warn!("imap.id.fields.{key} = true: no canned value defined, sending NIL");
                    NString::NIL
                }
            }
        } else {
            NString::NIL
        };

        params.push((ikey, nval));
    }

    Ok(Some(params))
}

/// Canned value for a well-known auto-`ID` key, [`None`] for any other
/// key, which is then sent as `NIL`.
fn canned_imap_id_value(key: &str) -> Option<&'static str> {
    match key {
        "name" => Some(env!("CARGO_PKG_NAME")),
        "version" => Some(env!("CARGO_PKG_VERSION")),
        "vendor" => Some("Pimalaya"),
        "support-url" => Some("https://github.com/pimalaya/himalaya-tui"),
        _ => None,
    }
}
