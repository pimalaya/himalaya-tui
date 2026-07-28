//! Cross-protocol [`EmailClient`] backing the interface.
//!
//! Mirrors the himalaya CLI: a single storage backend (the first
//! configured one, local before network) held in a [`BackendClient`]
//! enum, plus an optional SMTP transport for accounts whose storage
//! backend cannot send (IMAP, Maildir, m2dir). Each method matches the
//! active backend and calls its adapter (the per-protocol
//! `<proto>/backend.rs`), which takes and returns the shared
//! [`crate::email`] types.

use anyhow::{Result, anyhow, bail};

#[cfg(feature = "imap")]
use crate::imap::client::ImapClient;
#[cfg(feature = "jmap")]
use crate::jmap::client::JmapClient;
#[cfg(feature = "m2dir")]
use crate::m2dir::client::M2dirClient;
#[cfg(feature = "maildir")]
use crate::maildir::client::MaildirClient;
#[cfg(feature = "smtp")]
use crate::smtp::client::SmtpClient;
use crate::{
    config::AccountConfig,
    email::{
        envelope::Envelope,
        flag::{Flag, FlagOp},
        mailbox::Mailbox,
    },
};

/// Cross-protocol email client backing the interface. The test-only
/// `Default` client has no backend at all and can never perform I/O.
#[cfg_attr(test, derive(Default))]
pub struct EmailClient {
    storage: Option<BackendClient>,
    #[cfg(feature = "smtp")]
    smtp: Option<SmtpClient>,
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
    /// when one is configured. A failing SMTP connection is downgraded
    /// to a warning (sending stays unavailable) rather than aborting
    /// the whole session. Bails when no storage backend is usable.
    pub fn new(#[allow(unused_mut)] mut account_config: AccountConfig) -> Result<Self> {
        let storage = select_storage(&mut account_config)?;

        #[cfg(feature = "smtp")]
        let smtp = match account_config.smtp.take() {
            Some(config) => match SmtpClient::new(config) {
                Ok(smtp) => Some(smtp),
                Err(err) => {
                    log::warn!("SMTP backend disabled: {err}. Sending will be unavailable.");
                    None
                }
            },
            None => None,
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
    /// (JMAP), otherwise through the SMTP transport.
    #[cfg_attr(not(any(feature = "jmap", feature = "smtp")), allow(unused_variables))]
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        match &mut self.storage {
            #[cfg(feature = "jmap")]
            Some(BackendClient::Jmap(client)) => return client.send_message(raw),
            _ => {}
        }

        #[cfg(feature = "smtp")]
        if let Some(smtp) = &mut self.smtp {
            return smtp.send_message(raw);
        }

        bail!("No send-capable backend (JMAP) or SMTP is configured for this account")
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
