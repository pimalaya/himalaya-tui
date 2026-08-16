//! Himalaya TUI wrapper around [`io_imap::client::ImapClientStd`].
//!
//! The TUI opens the IMAP session once via [`ImapClient::new`] and then
//! calls the shared adapter methods (in the sibling `backend` module)
//! through the [`Deref`]/[`DerefMut`] passthrough to the inner client.

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
/// State is deliberately minimal: the retained shared-API methods
/// re-SELECT before every operation and never consult cached
/// capabilities, so nothing beyond the inner client needs to be kept.
pub struct ImapClient {
    inner: Inner,
}

impl ImapClient {
    /// Opens the IMAP connection (TCP/TLS/STARTTLS, greeting, SASL),
    /// offering the configured ALPN identifiers and honoring the
    /// account's auto-`ID` quirks.
    pub fn new(config: ImapConfig) -> Result<Self> {
        let tls = config
            .tls
            .into_tls(config.alpn.unwrap_or_else(default_alpn));
        let server = parse_imap_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // NOTE: a `unix://` sirup socket presents a pre-authenticated
            // session (the greeting is PREAUTH), so no SASL is negotiated.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server
                    .host_str()
                    .ok_or_else(|| anyhow!("Cannot derive host from IMAP server `{server}`"))?;
                // NOTE: url does not know the imap(s) default ports, so fall
                // back to the same scheme defaults io-imap connects with.
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

    /// Lightweight liveness check: issues an IMAP `NOOP` round-trip to
    /// confirm the connection is still usable and to poll for any
    /// pending untagged updates.
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
/// Accepts `imap`/`imaps://host[:port]`, a bare `host:port` or a bare
/// `host` (the last two default to `imaps://`, secure), or a
/// `unix:///path` socket for a local proxy such as sirup. Any other
/// scheme is rejected.
pub fn parse_imap_server(server: &str) -> Result<Url> {
    parse_server(server, "imaps", &["imap", "imaps", "unix"])
}

/// Resolves an [`ImapIdConfig`] into the wire-level parameter list
/// passed to the io-imap auth coroutines.
///
/// [`None`] when `auto = false`; otherwise a vec where each entry
/// maps the user-supplied key to either himalaya-tui's canned value
/// (when the user set `true` and the key is well-known) or `NIL`.
/// Unknown keys with `true` log a warning and fall back to `NIL`.
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

/// The value substituted for a well-known auto-`ID` key the user opted
/// into; [`None`] for any other key, which is then sent as `NIL`.
fn canned_imap_id_value(key: &str) -> Option<&'static str> {
    match key {
        "name" => Some(env!("CARGO_PKG_NAME")),
        "version" => Some(env!("CARGO_PKG_VERSION")),
        "vendor" => Some("Pimalaya"),
        "support-url" => Some("https://github.com/pimalaya/himalaya-tui"),
        _ => None,
    }
}
