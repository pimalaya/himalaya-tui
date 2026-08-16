//! himalaya-tui wrapper around [`io_jmap::client::JmapClientStd`] that
//! bundles the live JMAP session behind [`Deref`]/[`DerefMut`] so the
//! adapter methods in [`crate::jmap::backend`] can call the high-level
//! io_jmap methods directly.
//!
//! Built by the TUI model from a [`crate::config::JmapConfig`] block.

use std::ops::{Deref, DerefMut};

use anyhow::{Result, anyhow};
use base64::{Engine, prelude::BASE64_STANDARD};
use io_jmap::{client::JmapClientStd as Inner, rfc8621::mailbox::get::JmapMailboxGetOptions};
use secrecy::{ExposeSecret, SecretString};
use url::Url;

use crate::config::{JmapAuthConfig, JmapConfig, parse_server};

/// Live JMAP session paired with the resolved session-endpoint URL.
///
/// The URL is retained so [`JmapClient::ping`] can re-run the session
/// discovery against the same authority as a liveness check.
pub struct JmapClient {
    inner: Inner,
    /// Resolved JMAP session-endpoint URL, kept for [`JmapClient::ping`]
    /// and any later `session_get` refresh.
    url: Url,
    /// The original JMAP config block, kept so an auxiliary session can
    /// be opened against a blob URL living on another authority than the
    /// API one (see [`JmapClient::download_blob`]).
    config: JmapConfig,
    /// Lazily-fetched `(id, name)` pairs for every mailbox, used by
    /// [`JmapClient::resolve_mailbox_id`] to map the interface's
    /// human-facing mailbox names onto opaque JMAP ids. Cached for the
    /// client's lifetime so a copy or a move resolves both endpoints in
    /// a single `Mailbox/get`.
    mailbox_index: Option<Vec<(String, String)>>,
}

impl JmapClient {
    /// Establishes the JMAP session: TLS-connect to the configured
    /// server then fetch the session object (`/.well-known/jmap`
    /// discovery, primary accounts, upload/download URL templates).
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

    /// Liveness check: re-fetches the JMAP session object against the
    /// configured session endpoint. A successful `Session/get` proves
    /// the connection is still usable and refreshes the cached session
    /// (state, upload/download templates) in one round-trip.
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

    /// Maps a human mailbox name to its opaque JMAP id, for the shared
    /// client which otherwise addresses mailboxes by their id.
    ///
    /// A value that already matches a known id is returned verbatim (id
    /// passthrough, mirroring IMAP where the name *is* the id); an exact
    /// display-name match returns the mapped id (first match wins on the
    /// rare duplicate-name case); an unknown value is handed back as-is
    /// so the server surfaces the error. The mailbox index is fetched
    /// once (`Mailbox/get`) and cached.
    ///
    /// This lives here, on the himalaya-tui client, precisely so the
    /// backend operation methods (`list_envelopes`, `add_message`, …)
    /// stay pure id consumers: name resolution never happens inside them.
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

    /// Downloads a blob whose URL may live on a different authority than
    /// the JMAP API endpoint. Fastmail, for one, serves downloads from
    /// `*.fastmailusercontent.com` while the API is on `api.fastmail.com`.
    ///
    /// When the download host matches the API host the live session
    /// connection is reused; otherwise a fresh authenticated connection
    /// is opened to the download host. The API socket is *never* reused
    /// for a foreign host: doing so sends the download request to the
    /// API server, which (Fastmail) answers with a `302` to its docs
    /// page and fails the non-redirectable download.
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
/// Accepts a full `http`/`https://host[:port][/path]` URL, a bare
/// `host:port`, or a bare `host`; the last two default to `https://`
/// (secure). Any other scheme is rejected.
pub fn parse_jmap_server(server: &str) -> Result<Url> {
    parse_server(server, "https", &["http", "https"])
}

/// Converts a [`JmapAuthConfig`] into the pre-formatted HTTP
/// `Authorization` header value [`JmapClientStd::connect`] expects.
///
/// [`JmapClientStd::connect`]: io_jmap::client::JmapClientStd::connect
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

/// Whether two URLs share host and effective port, i.e. a live
/// connection to one can carry a request for the other.
fn same_authority(a: &Url, b: &Url) -> bool {
    a.host() == b.host() && a.port_or_known_default() == b.port_or_known_default()
}
