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
use chrono::{FixedOffset, TimeZone};
use io_maildir::{
    entry::MaildirFullEntry,
    flag::{MaildirFlag, MaildirFlags},
    maildir::{Maildir, MaildirSubdir},
};
use mime::{
    header::marker::{DATE, FROM, MESSAGE_ID, SUBJECT, TO},
    tree::cst::MimeCst,
    value::address::MimeAddressList,
};

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
            .map(|maildir| mailbox_from(self, maildir))
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
/// being the on-disk path and `name` the
/// [`MaildirClient::mailbox_name`].
fn mailbox_from(client: &MaildirClient, maildir: Maildir) -> Mailbox {
    Mailbox {
        name: client.mailbox_name(&maildir),
        id: maildir.path().to_string(),
        total: None,
        unread: None,
    }
}

/// Folds a fully-read Maildir entry into a shared [`Envelope`], parsing
/// the RFC 5322 headers and mapping the flags io-maildir resolved.
fn envelope_from_entry(entry: &MaildirFullEntry) -> Envelope {
    Envelope {
        id: entry.id().unwrap_or_default().to_string(),
        flags: entry.flags().iter().map(flag_from_maildir).collect(),
        ..envelope_from_raw(entry.contents())
    }
}

/// The envelope of a raw message, its id and flags left empty for the
/// entry to fill.
///
/// The body is split too, to say whether the message holds an attachment:
/// any part of the RFC 8621 §4.1.4 attachment list counts, an inline image
/// included.
fn envelope_from_raw(raw: &[u8]) -> Envelope {
    let parsed = MimeCst::parse(raw);
    let root = &parsed.root;

    let addresses =
        |list: Option<MimeAddressList>| list.map(|list| addresses_from(&list)).unwrap_or_default();

    let date = root
        .header::<DATE>()
        .filter(|date| date.is_valid())
        .and_then(|date| {
            let offset = FixedOffset::east_opt(i32::from(date.offset) * 60)?;
            offset.timestamp_opt(date.to_unix_seconds(), 0).single()
        });

    let message_id = root
        .header::<MESSAGE_ID>()
        .and_then(|ids| ids.into_iter().next())
        .and_then(|id| normalize_message_id(&id));

    Envelope {
        id: String::new(),
        message_id,
        flags: Default::default(),
        subject: root.header::<SUBJECT>().unwrap_or_default(),
        from: addresses(root.header::<FROM>()),
        to: addresses(root.header::<TO>()),
        date,
        size: raw.len() as u64,
        has_attachment: Some(!root.body_lists().attachments.is_empty()),
    }
}

/// Converts a mime-rs address list into shared [`Address`]es, groups
/// flattened, mailboxes with no address left out.
fn addresses_from(list: &MimeAddressList) -> Vec<Address> {
    list.mailboxes()
        .filter_map(|mailbox| {
            let email = mailbox.address.clone().filter(|email| !email.is_empty())?;
            Some(Address {
                name: mailbox.name.clone(),
                email,
            })
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

    /// Every envelope field is read off the raw message: encoded words
    /// decoded, a folded subject unfolded, groups flattened into their
    /// members, the date keeping its offset.
    #[test]
    fn the_envelope_is_read_off_the_raw_message() {
        let raw = concat!(
            "From: =?utf-8?q?Ren=C3=A9?= <rene@x.org>\r\n",
            "To: Friends: b@x.org, \"Carol C\" <c@x.org>;, d@x.org\r\n",
            "Subject: =?iso-8859-1?q?caf=E9?= au\r\n",
            " lait\r\n",
            "Date: Mon, 2 Nov 2026 10:20:30 +0100\r\n",
            "Message-ID: <abc@x.org>\r\n",
            "Content-Type: multipart/mixed; boundary=b\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/plain\r\n",
            "\r\n",
            "hi\r\n",
            "--b\r\n",
            "Content-Type: application/pdf\r\n",
            "Content-Disposition: attachment; filename=a.pdf\r\n",
            "\r\n",
            "%PDF\r\n",
            "--b--\r\n",
        );

        let envelope = envelope_from_raw(raw.as_bytes());
        let address = |name: Option<&str>, email: &str| Address {
            name: name.map(String::from),
            email: email.to_string(),
        };

        assert_eq!(envelope.subject, "café au lait");
        assert_eq!(envelope.from, [address(Some("René"), "rene@x.org")]);
        assert_eq!(
            envelope.to,
            [
                address(None, "b@x.org"),
                address(Some("Carol C"), "c@x.org"),
                address(None, "d@x.org"),
            ]
        );
        assert_eq!(
            envelope.date.map(|date| date.to_rfc3339()).as_deref(),
            Some("2026-11-02T10:20:30+01:00")
        );
        assert_eq!(envelope.message_id.as_deref(), Some("abc@x.org"));
        assert_eq!(envelope.size, raw.len() as u64);
        assert_eq!(envelope.has_attachment, Some(true));
    }

    /// A message with no attachment says so, and a missing header leaves
    /// its field empty rather than failing the listing.
    #[test]
    fn a_bare_message_has_empty_fields_and_no_attachment() {
        let envelope = envelope_from_raw(b"From: a@x.org\r\n\r\nhello\r\n");

        assert_eq!(envelope.subject, "");
        assert!(envelope.to.is_empty());
        assert_eq!(envelope.date, None);
        assert_eq!(envelope.message_id, None);
        assert_eq!(envelope.has_attachment, Some(false));
    }

    /// An image a `multipart/related` HTML body refers to counts as an
    /// attachment, as it always has on this backend.
    #[test]
    fn a_related_image_counts_as_an_attachment() {
        let raw = concat!(
            "From: a@x.org\r\n",
            "Content-Type: multipart/related; boundary=b\r\n",
            "\r\n",
            "--b\r\n",
            "Content-Type: text/html\r\n",
            "\r\n",
            "<img src=\"cid:logo\">\r\n",
            "--b\r\n",
            "Content-Type: image/png\r\n",
            "Content-ID: <logo>\r\n",
            "Content-Disposition: inline\r\n",
            "\r\n",
            "PNG\r\n",
            "--b--\r\n",
        );

        let envelope = envelope_from_raw(raw.as_bytes());

        assert_eq!(envelope.has_attachment, Some(true));
    }
}
