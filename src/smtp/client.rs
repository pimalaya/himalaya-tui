//! Himalaya TUI wrapper around [`io_smtp::client::SmtpClientStd`].
//!
//! SMTP send is stateless after auth, so the wrapper only holds the
//! live connection: subcommands that send a message build it up front
//! from the account's `[smtp]` block and hand it the raw bytes. The
//! send adapter lives in the sibling `backend` module.

use std::{
    net::Ipv4Addr,
    ops::{Deref, DerefMut},
};

use anyhow::Result;
use io_smtp::{client::SmtpClientStd as Inner, rfc5321::SmtpEhloDomain};
use pimalaya_stream::{sasl::Sasl, tls::Tls};

use crate::config::{SmtpConfig, parse_smtp_server};

/// SMTP client wrapping the inner stream for sending messages.
pub struct SmtpClient {
    inner: Inner,
}

impl SmtpClient {
    /// Opens the SMTP connection (TCP/TLS/STARTTLS, greeting, EHLO,
    /// SASL).
    pub fn new(config: SmtpConfig) -> Result<Self> {
        let mut tls: Tls = config.tls.try_into()?;
        tls.rustls.alpn = vec!["smtp".into()];
        let domain: SmtpEhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = parse_smtp_server(&config.server)?;
        let sasl: Option<Sasl> = config
            .sasl
            .map(|cfg| {
                let host = server.host_str().unwrap_or_default();
                // url does not know the smtp(s) default ports; gating on
                // port_or_known_default() would silently drop the whole SASL
                // config for a portless URL, opening an unauthenticated
                // session.
                let port = server
                    .port()
                    .unwrap_or_else(|| Inner::default_port(server.scheme()));
                cfg.try_into_sasl(host, port)
            })
            .transpose()?;
        let inner = Inner::connect(&server, &tls, config.starttls, domain, sasl)?;
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
