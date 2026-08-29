//! # JMAP client
//!
//! Wrapper around [`io_jmap::client::JmapClientStd`] holding the live
//! JMAP session behind [`Deref`]/[`DerefMut`], so the adapter methods in
//! [`crate::jmap::backend`] call the high-level io_jmap methods directly.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_STANDARD};
use io_jmap::{client::JmapClientStd as Inner, rfc8621::mailbox::get::JmapMailboxGetOptions};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::config::{JmapAuthConfig, JmapConfig, parse_server};

/// Live JMAP session paired with the resolved session-endpoint URL.
pub struct JmapClient {
    inner: Inner,
    /// Session-endpoint URL, re-fetched by [`JmapClient::ping`].
    url: Url,
    /// Original JMAP config block.
    ///
    /// Kept so a blob URL living on another authority than the API one
    /// can get a session of its own (see [`JmapClient::download_blob`]).
    config: JmapConfig,
    /// Lazily-fetched `(id, name)` pairs for every mailbox.
    ///
    /// Cached for the client's lifetime so a copy or a move resolves
    /// both of its endpoints in a single `Mailbox/get`.
    mailbox_index: Option<Vec<(String, String)>>,
}

impl JmapClient {
    /// Establishes the JMAP session.
    ///
    /// TLS-connects to the configured server, then fetches the session
    /// object.
    pub fn new(config: JmapConfig) -> Result<Self> {
        let tls = config
            .tls
            .clone()
            .into_tls(config.alpn.clone().unwrap_or_else(Inner::default_alpn));
        let http_auth = jmap_http_auth(config.auth.clone())?;
        let url = parse_jmap_server(&config.server)?;

        let mut inner = Inner::connect(&url, &tls, http_auth)?;
        inner.session_get(&url)?;

        Ok(Self {
            inner,
            url,
            config,
            mailbox_index: None,
        })
    }

    /// Liveness check: re-fetches the JMAP session object.
    ///
    /// A successful `Session/get` proves the connection is still usable
    /// and refreshes the cached session in one round-trip.
    pub fn ping(&mut self) -> Result<()> {
        self.inner.session_get(&self.url)?;
        Ok(())
    }

    /// The sending identity pinned by the configuration, if any.
    pub fn identity_id(&self) -> Option<&str> {
        self.config.identity_id.as_deref()
    }

    /// The drafts mailbox pinned by the configuration, if any.
    pub fn drafts_mailbox_id(&self) -> Option<&str> {
        self.config.drafts_mailbox_id.as_deref()
    }

    /// Maps a human mailbox name to its opaque JMAP id.
    ///
    /// The index is fetched once and cached. A known id passes through
    /// verbatim, mirroring IMAP where the name is the id, and an unknown
    /// value is handed back as-is so the server surfaces the error.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        if self.mailbox_index.is_none() {
            let output = self.mailbox_get(JmapMailboxGetOptions {
                ids: None,
                properties: None,
            })?;
            let index = output
                .mailboxes
                .into_iter()
                .filter_map(|mailbox| Some((mailbox.id?, mailbox.name.unwrap_or_default())))
                .collect();
            self.mailbox_index = Some(index);
        }

        let index = self.mailbox_index.as_deref().unwrap_or_default();

        if index.iter().any(|(id, _)| id == mailbox) {
            return Ok(mailbox.to_string());
        }

        if let Some((id, _)) = index.iter().find(|(_, name)| name == mailbox) {
            return Ok(id.clone());
        }

        Ok(mailbox.to_string())
    }

    /// Downloads a blob, opening its own session on a foreign authority.
    ///
    /// Fastmail, for one, serves downloads from fastmailusercontent.com
    /// while the API is on api.fastmail.com. Reusing the API socket for
    /// a foreign host earns a 302 to its docs page, failing the
    /// non-redirectable download.
    pub fn download_blob(&mut self, download_url: &Url) -> Result<Vec<u8>> {
        let api_url = {
            let session = self
                .session()
                .ok_or_else(|| anyhow!("JMAP session is missing"))?;
            session.api_url.clone()
        };

        if same_authority(&api_url, download_url) {
            return Ok(self.blob_download(download_url)?);
        }

        let tls = self
            .config
            .tls
            .clone()
            .into_tls(self.config.alpn.clone().unwrap_or_else(Inner::default_alpn));
        let http_auth = jmap_http_auth(self.config.auth.clone())?;
        let mut download_client = Inner::connect(download_url, &tls, http_auth)?;

        Ok(download_client.blob_download(download_url)?)
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

/// Parses the JMAP `server` field into a [`Url`].
///
/// Accepts a full http or https URL, a bare `host:port` or a bare
/// `host`, the last two defaulting to https. Any other scheme is
/// rejected.
pub fn parse_jmap_server(server: &str) -> Result<Url> {
    parse_server(server, "https", &["http", "https"])
}

/// Formats a [`JmapAuthConfig`] into an HTTP `Authorization` value.
pub fn jmap_http_auth(config: JmapAuthConfig) -> Result<SecretString> {
    match config {
        JmapAuthConfig::Header(token) => Ok(token.get()?),
        JmapAuthConfig::Bearer { token } => {
            let token = token.get()?;
            Ok(format!("Bearer {}", token.expose_secret()).into())
        }
        JmapAuthConfig::Basic { username, password } => {
            let creds = format!("{}:{}", username, password.get()?.expose_secret());
            let encoded = BASE64_STANDARD.encode(creds.into_bytes());
            Ok(format!("Basic {encoded}").into())
        }
    }
}

/// Whether two URLs share host and effective port.
///
/// Same authority means one live connection can carry both requests.
fn same_authority(a: &Url, b: &Url) -> bool {
    a.host() == b.host() && a.port_or_known_default() == b.port_or_known_default()
}
