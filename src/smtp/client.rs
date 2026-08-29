//! # SMTP client
//!
//! Wrapper around [`io_smtp::client::SmtpClientStd`] holding the live
//! connection.
//!
//! SMTP send is stateless after auth, so nothing else needs keeping:
//! senders build the client up front from the account's `[smtp]` block
//! and hand it the raw bytes.

use std::{
    net::Ipv4Addr,
    ops::{Deref, DerefMut},
};

use anyhow::{Result, anyhow};
use io_sasl::mechanism::Sasl;
use io_smtp::{
    client::SmtpClientStd as Inner, rfc5321::SmtpEhloDomain, session::SmtpSessionOpenOptions,
};
use url::Url;

use crate::config::{SmtpConfig, parse_server};

/// SMTP client wrapping the inner stream for sending messages.
pub struct SmtpClient {
    inner: Inner,
}

impl SmtpClient {
    /// Opens the SMTP connection: TCP/TLS/STARTTLS, greeting, EHLO, SASL.
    pub fn new(config: SmtpConfig) -> Result<Self> {
        let tls = config
            .tls
            .into_tls(config.alpn.unwrap_or_else(Inner::default_alpn));
        let domain: SmtpEhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = parse_smtp_server(&config.server)?;
        let sasl: Option<Sasl> = match config.sasl {
            // NOTE: a `unix://` sirup socket presents a pre-authenticated
            // session, so no SASL is negotiated over it.
            Some(_) if server.scheme() == "unix" => None,
            Some(cfg) => {
                let host = server
                    .host_str()
                    .ok_or_else(|| anyhow!("Cannot derive host from SMTP server `{server}`"))?;
                // NOTE: url knows no smtp(s) default port, so match
                // io-smtp's own connection defaults (465 for smtps).
                let port = server
                    .port()
                    .unwrap_or(Inner::default_port(server.scheme()));
                Some(cfg.try_into_sasl(host, port)?)
            }
            None => None,
        };
        let opts = SmtpSessionOpenOptions {
            starttls: config.starttls,
        };
        let (inner, _capabilities) = Inner::connect(&server, &tls, domain, sasl, opts)?;

        Ok(Self { inner })
    }
}

impl Deref for SmtpClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for SmtpClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Parses an SMTP server string into a URL.
///
/// Accepts an smtp or smtps URL, a bare `host:port` or a bare `host`
/// (the last two defaulting to smtps), or a unix socket for a local
/// proxy such as sirup. Any other scheme is rejected.
pub fn parse_smtp_server(server: &str) -> Result<Url> {
    parse_server(server, "smtps", &["smtp", "smtps", "unix"])
}
