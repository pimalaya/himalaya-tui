//! himalaya-tui wrapper around [`io_jmap::client::JmapClientStd`] that
//! bundles the live JMAP session behind [`Deref`]/[`DerefMut`] so the
//! adapter methods in [`crate::jmap::backend`] can call the high-level
//! io_jmap methods directly.
//!
//! Built by the TUI model from a [`crate::config::JmapConfig`] block.

use std::ops::{Deref, DerefMut};

use anyhow::Result;
use io_jmap::client::JmapClientStd as Inner;
use url::Url;

use crate::config::{JmapConfig, jmap_http_auth, parse_jmap_server};

/// Live JMAP session paired with the resolved session-endpoint URL.
///
/// The URL is retained so [`JmapClient::ping`] can re-run the session
/// discovery against the same authority as a liveness check.
pub struct JmapClient {
    inner: Inner,
    /// Resolved JMAP session-endpoint URL, kept for [`JmapClient::ping`]
    /// and any later `session_get` refresh.
    url: Url,
    /// Configured sending identity, overriding the one the send path
    /// would otherwise resolve from the live session.
    pub identity_id: Option<String>,
    /// Configured drafts mailbox, overriding the one the send path
    /// would otherwise resolve from the live session.
    pub drafts_mailbox_id: Option<String>,
}

impl JmapClient {
    /// Establishes the JMAP session: TLS-connect to the configured
    /// server then fetch the session object (`/.well-known/jmap`
    /// discovery, primary accounts, upload/download URL templates).
    pub fn new(config: JmapConfig) -> Result<Self> {
        let tls = config.tls.into_tls(config.alpn);
        let http_auth = jmap_http_auth(config.auth)?;
        let url = parse_jmap_server(&config.server)?;

        let mut inner = Inner::connect(&url, &tls, http_auth)?;
        inner.session_get(&url)?;

        Ok(Self {
            inner,
            url,
            identity_id: config.identity_id,
            drafts_mailbox_id: config.drafts_mailbox_id,
        })
    }

    /// Liveness check: re-fetches the JMAP session object against the
    /// configured session endpoint. A successful `Session/get` proves
    /// the connection is still usable and refreshes the cached session
    /// (state, upload/download templates) in one round-trip.
    pub fn ping(&mut self) -> Result<()> {
        self.inner.session_get(&self.url)?;
        Ok(())
    }
}

impl Deref for JmapClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for JmapClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
