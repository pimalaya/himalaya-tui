//! Cross-protocol [`EmailClient`] backing the interface.
//!
//! Mirrors the himalaya CLI: a single storage backend (the first
//! configured one, local before network) held in a [`BackendClient`]
//! enum, plus an optional SMTP transport for accounts whose storage
//! backend cannot send (IMAP, Maildir, m2dir). Each method matches the
//! active backend and calls its adapter (the per-protocol
//! `<proto>/backend.rs`), which takes and returns the shared
//! [`crate::email`] types.

#[cfg(feature = "smtp")]
use std::mem;

use anyhow::{Result, anyhow, bail};

#[cfg(feature = "imap")]
use crate::imap::client::ImapClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
#[cfg(feature = "m2dir")]
use crate::m2dir::client::M2dirClient;
#[cfg(feature = "maildir")]
use crate::maildir::client::MaildirClient;
use crate::{
    config::AccountConfig,
    email::{
        envelope::Envelope,
        flag::{Flag, FlagOp},
        mailbox::Mailbox,
    },
};
#[cfg(feature = "smtp")]
use crate::{config::SmtpConfig, smtp::client::SmtpClient};

/// Cross-protocol email client backing the interface.
pub struct EmailClient {
    storage: Option<BackendClient>,
    #[cfg(feature = "smtp")]
    smtp: SmtpTransport,
}

/// The SMTP transport slot, connected lazily on the first send so that
/// a reading session never opens an SMTP connection.
#[cfg(feature = "smtp")]
enum SmtpTransport {
    /// No SMTP configured for this account.
    Absent,
    /// Configured but not yet connected.
    Pending(Box<SmtpConfig>),
    /// Connected.
    Ready(SmtpClient),
}

/// The active storage backend: exactly one of the compiled-in
/// per-protocol clients.
enum BackendClient {
    #[cfg(feature = "imap")]
    Imap(Box<ImapClient>),
    #[cfg(feature = "jmap")]
    Jmap(Box<JmapClient>),
    #[cfg(feature = "maildir")]
    Maildir(Box<MaildirClient>),
    #[cfg(feature = "m2dir")]
    M2dir(Box<M2dirClient>),
}

impl EmailClient {
    /// Opens the connections for the account: the first configured
    /// storage backend (local before network), plus an SMTP transport
    /// when one is configured. Bails when no storage backend is usable.
    pub fn new(#[allow(unused_mut)] mut account_config: AccountConfig) -> Result<Self> {
        let storage = select_storage(&mut account_config)?;

        // NOTE: kept unconnected here, so a session that only reads
        // opens no SMTP connection (it also lets a single-session proxy
        // such as sirup serve the storage backend without a second
        // client).
        #[cfg(feature = "smtp")]
        let smtp = match account_config.smtp.take() {
            Some(config) => SmtpTransport::Pending(Box::new(config)),
            None => SmtpTransport::Absent,
        };

        if storage.is_none() {
            bail!("No usable storage backend is configured for this account");
        }

        Ok(Self {
            storage,
            #[cfg(feature = "smtp")]
            smtp,
        })
    }

    /// Lightweight liveness check against the active storage backend.
    pub fn ping(&mut self) -> Result<()> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.ping(),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.ping(),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.ping(),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.ping(),
        }
    }

    /// Lists every mailbox available to the account.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.list_mailboxes(with_counts),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.list_mailboxes(with_counts),
        }
    }

    /// Lists envelopes from `mailbox`.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => {
                client.list_envelopes(mailbox, page, page_size, with_attachment)
            }
        }
    }

    /// Fetches one message's raw RFC 5322 bytes.
    pub fn get_message(&mut self, mailbox: &str, id: &str) -> Result<Vec<u8>> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.get_message(mailbox, id),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.get_message(mailbox, id),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.get_message(mailbox, id),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.get_message(mailbox, id),
        }
    }

    /// Adds, sets, or removes `flags` on a message id set in `mailbox`.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.store_flags(mailbox, ids, flags, op),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.store_flags(mailbox, ids, flags, op),
        }
    }

    /// Adds `raw` to `mailbox` with `flags`. Returns the created id.
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let mailbox = self.resolve_mailbox_id(mailbox)?;
        let mailbox = mailbox.as_str();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.add_message(mailbox, flags, raw),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.add_message(mailbox, flags, raw),
        }
    }

    /// Copies a message id set from `from` to `to`.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let from = self.resolve_mailbox_id(from)?;
        let from = from.as_str();
        let to = self.resolve_mailbox_id(to)?;
        let to = to.as_str();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.copy_messages(from, to, ids),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.copy_messages(from, to, ids),
        }
    }

    /// Moves a message id set from `from` to `to`.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let from = self.resolve_mailbox_id(from)?;
        let from = from.as_str();
        let to = self.resolve_mailbox_id(to)?;
        let to = to.as_str();

        match self.storage_mut()? {
            #[cfg(feature = "imap")]
            BackendClient::Imap(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "maildir")]
            BackendClient::Maildir(client) => client.move_messages(from, to, ids),
            #[cfg(feature = "m2dir")]
            BackendClient::M2dir(client) => client.move_messages(from, to, ids),
        }
    }

    /// Sends `raw`: through the storage backend when it can send itself
    /// (JMAP), otherwise through the SMTP transport, connecting it on
    /// this first use.
    #[cfg_attr(not(any(feature = "jmap", feature = "smtp")), allow(unused_variables))]
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        match &mut self.storage {
            #[cfg(feature = "jmap")]
            Some(BackendClient::Jmap(client)) => return client.send_message(raw),
            _ => {}
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp) = self.smtp_mut()? {
            return smtp.send_message(raw);
        }

        bail!("No send-capable backend (JMAP) or SMTP is configured for this account")
    }

    /// Maps the interface's human mailbox name onto the backend-native
    /// id the operation methods expect. Identity for every backend whose
    /// name already is its id (IMAP, Maildir, m2dir); JMAP resolves the
    /// opaque mailbox id via a cached `Mailbox/get`. Applied by the
    /// mailbox-addressing methods above before they dispatch, so each
    /// per-protocol adapter only ever receives ids. Idempotent: an
    /// already-resolved id passes through unchanged.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        match self.storage_mut()? {
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.resolve_mailbox_id(mailbox),
            #[allow(unreachable_patterns)]
            _ => Ok(mailbox.to_string()),
        }
    }

    /// The SMTP transport, connected on this first use. [`None`] when
    /// the account configures no `[smtp]` block.
    #[cfg(feature = "smtp")]
    fn smtp_mut(&mut self) -> Result<Option<&mut SmtpClient>> {
        if let SmtpTransport::Pending(_) = &self.smtp {
            let SmtpTransport::Pending(config) =
                mem::replace(&mut self.smtp, SmtpTransport::Absent)
            else {
                unreachable!()
            };
            self.smtp = SmtpTransport::Ready(SmtpClient::new(*config)?);
        }

        Ok(match &mut self.smtp {
            SmtpTransport::Ready(client) => Some(client),
            _ => None,
        })
    }

    fn storage_mut(&mut self) -> Result<&mut BackendClient> {
        self.storage
            .as_mut()
            .ok_or_else(|| anyhow!("No storage backend is configured for this account"))
    }
}

/// Picks the storage backend for the account: the first configured one,
/// local before network to match the retired io-email dispatcher's read
/// priority.
#[cfg_attr(
    not(any(
        feature = "maildir",
        feature = "m2dir",
        feature = "jmap",
        feature = "imap"
    )),
    allow(unused_variables)
)]
fn select_storage(account_config: &mut AccountConfig) -> Result<Option<BackendClient>> {
    #[cfg(feature = "maildir")]
    if let Some(config) = account_config.maildir.take() {
        return Ok(Some(BackendClient::Maildir(Box::new(MaildirClient::new(
            config,
        )))));
    }

    #[cfg(feature = "m2dir")]
    if let Some(config) = account_config.m2dir.take() {
        return Ok(Some(BackendClient::M2dir(Box::new(M2dirClient::new(
            config,
        )))));
    }

    #[cfg(feature = "jmap")]
    if let Some(config) = account_config.jmap.take() {
        return Ok(Some(BackendClient::Jmap(Box::new(JmapClient::new(
            config,
        )?))));
    }

    #[cfg(feature = "imap")]
    if let Some(config) = account_config.imap.take() {
        return Ok(Some(BackendClient::Imap(Box::new(ImapClient::new(
            config,
        )?))));
    }

    Ok(None)
}
