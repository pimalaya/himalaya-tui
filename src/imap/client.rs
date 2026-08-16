//! Himalaya TUI wrapper around [`io_imap::client::ImapClientStd`].
//!
//! The TUI opens the IMAP session once via [`ImapClient::new`] and then
//! calls the shared adapter methods (in the sibling `backend` module)
//! through the [`Deref`]/[`DerefMut`] passthrough to the inner client.

use std::ops::{Deref, DerefMut};

use anyhow::Result;
use io_imap::{
    client::{ImapClient as _, ImapClientStd as Inner},
    session::ImapSessionOpenOptions,
};
use io_sasl::mechanism::Sasl;

use crate::config::{ImapConfig, parse_imap_server, resolve_auto_id_params};

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
    /// pinning the `imap` ALPN identifier and honoring the account's
    /// auto-`ID` quirks.
    pub fn new(config: ImapConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let server = parse_imap_server(&config.server)?;
        let sasl: Option<Sasl> = config
            .sasl
            .map(|cfg| {
                let host = server.host_str().unwrap_or_default();
                // url does not know the imap(s) default ports; gating on
                // port_or_known_default() would silently drop the whole SASL
                // config for a portless URL, opening an unauthenticated
                // session.
                let port = server
                    .port()
                    .unwrap_or_else(|| io_imap::client::default_port(server.scheme()));
                cfg.try_into_sasl(host, port)
            })
            .transpose()?;
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
