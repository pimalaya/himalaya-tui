//! # Maildir backend
//!
//! Maildir adapter for the shared cross-protocol client: thin glue over
//! [`MaildirClient`], whose io_maildir client already walks the on-disk
//! entries.
//!
//! Each method takes and returns the shared [`crate::email`] types, so
//! the only real work is converting between those and io_maildir's
//! entries and flags.

use std::{cmp::Reverse, path::Path};

use anyhow::Result;
use chrono::DateTime;
use io_maildir::{
    entry::MaildirFullEntry,
    flag::{MaildirFlag, MaildirFlags},
    maildir::{Maildir, MaildirSubdir},
};
use mail_parser::Address as MailParserAddress;

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, EnvelopeList, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
    },
    maildir::client::MaildirClient,
};

impl MaildirClient {
    /// Checks liveness, a no-op: a filesystem backend has no session to
    /// open and no server to reach.
    pub fn ping(&mut self) -> Result<()> {
        Ok(())
    }

    /// Lists every Maildir under the configured root, sorted by name.
    ///
    /// `with_counts` is ignored: Maildir surfaces no cheap count, they
    /// would cost a full directory walk per mailbox.
    pub fn list_mailboxes(&self, _with_counts: bool) -> Result<Vec<Mailbox>> {
        let mut mailboxes: Vec<Mailbox> = self
            .list_maildirs()?
            .into_iter()
            .map(mailbox_from)
            .collect();
        mailboxes.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, sorted by `Date:` descending then
    /// paginated. `with_attachment` is always honoured, the body being
    /// parsed either way.
    pub fn list_envelopes(
        &self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        _with_attachment: bool,
    ) -> Result<EnvelopeList> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entries: Vec<_> = self.list_entries(maildir.clone())?.into_iter().collect();
        let fulls = self.read_entries(&maildir, &entries)?;

        let mut envelopes: Vec<Envelope> = fulls.iter().map(envelope_from_entry).collect();
        envelopes.sort_by_key(|envelope| Reverse(envelope.date));

        // NOTE: the whole mailbox is read to answer one page, so the
        // count is exact and costs nothing the read has not paid for.
        let total = envelopes.len().try_into().unwrap_or(u32::MAX);

        Ok(EnvelopeList {
            envelopes: paginate(envelopes, page, page_size),
            total,
        })
    }

    /// Adds or removes `flags` on a Maildir id set.
    pub fn store_flags(
        &self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let maildir_flags = flags_to_maildir(flags);

        for id in ids {
            match op {
                FlagOp::Add => self.add_flags(maildir.clone(), *id, maildir_flags.clone())?,
                FlagOp::Remove => self.remove_flags(maildir.clone(), *id, maildir_flags.clone())?,
            }
        }

        Ok(())
    }

    /// Reads one message's raw RFC 5322 bytes from `mailbox`.
    pub fn get_message(&self, mailbox: &str, id: &str) -> Result<Vec<u8>> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let entry = self.get(maildir, id)?;
        Ok(entry.contents().to_vec())
    }

    /// Stores `raw` under `mailbox`'s `cur/` with `flags`, returning the
    /// assigned Maildir id.
    pub fn add_message(&self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let maildir = self.resolve_maildir(Path::new(mailbox))?;
        let maildir_flags = flags_to_maildir(flags);
        let (id, _path) = self.store(maildir, MaildirSubdir::Cur, maildir_flags, raw)?;
        Ok(id)
    }

    /// Copies every id from `from` to `to`.
    pub fn copy_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;

        for id in ids {
            self.copy(*id, source.clone(), target.clone(), None)?;
        }

        Ok(())
    }

    /// Moves every id from `from` to `to`.
    pub fn move_messages(&self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let source = self.resolve_maildir(Path::new(from))?;
        let target = self.resolve_maildir(Path::new(to))?;

        for id in ids {
            self.r#move(*id, source.clone(), target.clone(), None)?;
        }

        Ok(())
    }
}

/// Converts one [`Maildir`] into the shared [`Mailbox`] shape, `id`
/// being the on-disk path and `name` its last segment.
fn mailbox_from(maildir: Maildir) -> Mailbox {
    Mailbox {
        id: maildir.path().to_string(),
        name: maildir.name().unwrap_or("").to_string(),
        total: None,
        unread: None,
    }
}

/// Folds a fully-read Maildir entry into a shared [`Envelope`], parsing
/// the RFC 5322 headers and mapping the flags io-maildir resolved.
fn envelope_from_entry(entry: &MaildirFullEntry) -> Envelope {
    let id = entry.id().unwrap_or_default().to_string();
    let flags = entry.flags().iter().map(flag_from_maildir).collect();
    let size = entry.contents().len() as u64;
    let parsed = entry.parsed();

    let subject = parsed
        .as_ref()
        .and_then(|m| m.subject())
        .unwrap_or_default()
        .to_string();

    let from = parsed
        .as_ref()
        .and_then(|m| m.from())
        .map(addresses_from)
        .unwrap_or_default();

    let to = parsed
        .as_ref()
        .and_then(|m| m.to())
        .map(addresses_from)
        .unwrap_or_default();

    let date = parsed
        .as_ref()
        .and_then(|m| m.date())
        .and_then(|d| DateTime::parse_from_rfc3339(&d.to_rfc3339()).ok());

    let has_attachment = parsed.as_ref().map(|m| m.attachment_count() > 0);

    let message_id = parsed
        .as_ref()
        .and_then(|m| m.message_id())
        .and_then(normalize_message_id);

    Envelope {
        id,
        message_id,
        flags,
        subject,
        from,
        to,
        date,
        size,
        has_attachment,
    }
}

/// Converts a mail-parser address group into shared [`Address`]es.
fn addresses_from(addrs: &MailParserAddress<'_>) -> Vec<Address> {
    addrs
        .clone()
        .into_list()
        .into_iter()
        .filter_map(|a| {
            let email = a.address?.into_owned();
            if email.is_empty() {
                return None;
            }
            let name = a.name.map(|s| s.into_owned());
            Some(Address { name, email })
        })
        .collect()
}

/// Maps a shared [`Flag`] to a [`MaildirFlag`], a keyword with no
/// letter going through the dovecot-keywords sidecar.
fn flag_to_maildir(flag: &Flag) -> MaildirFlag {
    match flag.iana() {
        Some(IanaFlag::Seen) => MaildirFlag::Seen,
        Some(IanaFlag::Answered) => MaildirFlag::Replied,
        Some(IanaFlag::Flagged) => MaildirFlag::Flagged,
        Some(IanaFlag::Draft) => MaildirFlag::Draft,
        Some(IanaFlag::Deleted) => MaildirFlag::Trashed,
        Some(IanaFlag::Forwarded) => MaildirFlag::Passed,
        Some(_) | None => MaildirFlag::Keyword(flag.raw().to_string()),
    }
}

/// Converts a shared flag slice into [`MaildirFlags`].
fn flags_to_maildir(flags: &[Flag]) -> MaildirFlags {
    flags.iter().map(flag_to_maildir).collect()
}

/// Maps a [`MaildirFlag`] back to a shared [`Flag`], a dovecot or
/// header keyword keeping the raw spelling it was stored under.
fn flag_from_maildir(flag: &MaildirFlag) -> Flag {
    match flag {
        MaildirFlag::Seen => Flag::from_iana(IanaFlag::Seen),
        MaildirFlag::Replied => Flag::from_iana(IanaFlag::Answered),
        MaildirFlag::Flagged => Flag::from_iana(IanaFlag::Flagged),
        MaildirFlag::Draft => Flag::from_iana(IanaFlag::Draft),
        MaildirFlag::Trashed => Flag::from_iana(IanaFlag::Deleted),
        MaildirFlag::Passed => Flag::from_iana(IanaFlag::Forwarded),
        MaildirFlag::Keyword(keyword) => Flag::from_raw(keyword),
    }
}

/// 1-indexed in-memory pagination: a `None` page size returns the whole
/// slice, a zero size or a page past the end returns nothing.
fn paginate<T>(items: Vec<T>, page: Option<u32>, page_size: Option<u32>) -> Vec<T> {
    let Some(size) = page_size else {
        return items;
    };
    if size == 0 {
        return Vec::new();
    }
    let page = page.unwrap_or(1).max(1);
    let skip = ((page - 1) as usize).saturating_mul(size as usize);
    if skip >= items.len() {
        return Vec::new();
    }
    items.into_iter().skip(skip).take(size as usize).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The two mappings are each other's inverse, keyword included, so
    /// a flag read off disk and written back lands on the same letter
    /// or the same dovecot slot.
    #[test]
    fn maildir_flags_round_trip_through_the_shared_flag() {
        let flags = [
            MaildirFlag::Seen,
            MaildirFlag::Replied,
            MaildirFlag::Flagged,
            MaildirFlag::Draft,
            MaildirFlag::Trashed,
            MaildirFlag::Passed,
            MaildirFlag::keyword("NonJunk"),
        ];

        for flag in flags {
            assert_eq!(flag_to_maildir(&flag_from_maildir(&flag)), flag);
        }
    }

    /// Pagination is 1-indexed and each page is the next slice, the
    /// last one short: a mailbox larger than a page is only reachable
    /// if the slice moves.
    #[test]
    fn each_page_is_the_next_slice() {
        let items: Vec<u8> = (1..=12).collect();
        let page = |n| paginate(items.clone(), Some(n), Some(5));

        assert_eq!(page(1), [1, 2, 3, 4, 5]);
        assert_eq!(page(2), [6, 7, 8, 9, 10]);
        assert_eq!(page(3), [11, 12]);
        assert!(page(4).is_empty(), "nothing is left to page into");
    }

    /// A keyword io-maildir resolved keeps the spelling it was stored
    /// under, whether or not IANA knows the name.
    #[test]
    fn a_resolved_keyword_keeps_its_wire_spelling() {
        let junk = flag_from_maildir(&MaildirFlag::keyword("$Junk"));
        assert_eq!(junk.raw(), "$Junk");
        assert_eq!(junk.iana(), Some(IanaFlag::Junk));

        let custom = flag_from_maildir(&MaildirFlag::keyword("NonJunk"));
        assert_eq!(custom.raw(), "NonJunk");
        assert_eq!(custom.iana(), None);
    }
}
