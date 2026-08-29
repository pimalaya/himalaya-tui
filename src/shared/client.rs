//! # Client
//!
//! The cross-protocol [`EmailClient`] backing the interface: one storage
//! backend, plus an optional SMTP transport for accounts whose backend
//! cannot send (IMAP, Maildir).
//!
//! Mirrors the himalaya CLI: each method matches the active backend and
//! forwards to its adapter, the protocol module's backend submodule, which
//! takes and returns the shared [`crate::email`] types.

#[cfg(feature = "smtp")]
use std::mem;

use anyhow::{Result, anyhow, bail};

#[cfg(feature = "imap")]
use crate::imap::client::ImapClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
#[cfg(feature = "maildir")]
use crate::maildir::client::MaildirClient;
use crate::{
    config::AccountConfig,
    email::{
        envelope::EnvelopeList,
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

/// The SMTP transport slot, connected lazily on the first send.
#[cfg(feature = "smtp")]
enum SmtpTransport {
    /// No SMTP configured for this account.
    Absent,
    /// Configured but not yet connected.
    Pending(Box<SmtpConfig>),
    /// Connected.
    Ready(SmtpClient),
}

/// The active storage backend, one of the compiled-in clients.
enum BackendClient {
    #[cfg(feature = "imap")]
    Imap(Box<ImapClient>),
    #[cfg(feature = "jmap")]
    Jmap(Box<JmapClient>),
    #[cfg(feature = "maildir")]
    Maildir(Box<MaildirClient>),
}

impl EmailClient {
    /// Opens the account's connections, bailing when no storage backend
    /// is usable.
    ///
    /// The storage backend is the first configured one, local before
    /// network; an SMTP transport joins it when the account declares one.
    pub fn new(#[allow(unused_mut)] mut account_config: AccountConfig) -> Result<Self> {
        let storage = select_storage(&mut account_config)?;

        // NOTE: staying unconnected lets a read-only session open no SMTP
        // connection, and a single-session proxy such as sirup serve the
        // storage backend without a second client.
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
        }
    }

    /// Lists one page of envelopes from `mailbox`, with its total.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<EnvelopeList> {
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
        }
    }

    /// Adds or removes `flags` on a message id set in `mailbox`.
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
        }
    }

    /// Adds `raw` to `mailbox` with `flags`, returning the created id.
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
        }
    }

    /// Sends `raw` through the storage backend when it can send (JMAP),
    /// otherwise over the SMTP transport.
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

    /// Maps a human mailbox name onto the backend-native id, idempotently.
    ///
    /// Identity wherever the name already is the id (IMAP, Maildir), a
    /// cached `Mailbox/get` on JMAP. The methods above apply it before
    /// dispatching, so an adapter only ever receives ids.
    pub fn resolve_mailbox_id(&mut self, mailbox: &str) -> Result<String> {
        match self.storage_mut()? {
            #[cfg(feature = "jmap")]
            BackendClient::Jmap(client) => client.resolve_mailbox_id(mailbox),
            #[allow(unreachable_patterns)]
            _ => Ok(mailbox.to_string()),
        }
    }

    /// The SMTP transport, connected on this first use, [`None`] if absent.
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

/// Picks the account's storage backend, the first configured one.
///
/// Local before network, matching the retired io-email dispatcher's read
/// priority.
#[cfg_attr(
    not(any(feature = "maildir", feature = "jmap", feature = "imap")),
    allow(unused_variables)
)]
fn select_storage(account_config: &mut AccountConfig) -> Result<Option<BackendClient>> {
    #[cfg(feature = "maildir")]
    if let Some(config) = account_config.maildir.take() {
        return Ok(Some(BackendClient::Maildir(Box::new(MaildirClient::new(
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
