//! IMAP adapter for the shared cross-protocol client.
//!
//! Thin glue over [`ImapClient`], which already wraps io_imap's
//! high-level session (`select`, `fetch`, `store`, `copy`, `move`,
//! `append`, `list`, `status`). Each method takes and returns the TUI's
//! shared [`crate::email`] types; the only real work is converting
//! between those and io_imap's wire types, adapted from the retired
//! io-email IMAP drivers.

use std::{collections::BTreeSet, num::NonZeroU32, str::from_utf8};

use anyhow::{Result, anyhow, bail};
use chrono::{DateTime, FixedOffset};
use io_imap::{
    client::ImapClient as _,
    rfc3501::{
        append::ImapMessageAppendOptions, copy::ImapMessageCopyOptions,
        fetch::ImapMessageFetchOptions, search::ImapMessageSearchOptions,
        select::ImapMailboxSelectOptions, store::ImapMessageStoreOptions,
    },
    rfc6851::r#move::ImapMessageMoveOptions,
    types::{
        body::BodyStructure,
        core::{AString, Atom, QuotedChar, Vec1},
        envelope::Address as ImapAddress,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        flag::{Flag as ImapFlag, FlagFetch, FlagNameAttribute, StoreType},
        mailbox::{ListMailbox, Mailbox as ImapMailbox},
        search::SearchKey,
        sequence::SequenceSet,
        status::{StatusDataItem, StatusDataItemName},
    },
};
use mail_parser::MessageParser;
use rfc2047_decoder::{Decoder, RecoverStrategy};

use crate::{
    email::{
        address::Address,
        envelope::{Envelope, normalize_message_id},
        flag::{Flag, FlagOp, IanaFlag},
        mailbox::Mailbox,
    },
    imap::client::ImapClient,
};

impl ImapClient {
    /// Lists every selectable mailbox. With `with_counts`, follows each
    /// row with a STATUS to populate totals and unread counts.
    pub fn list_mailboxes(&mut self, with_counts: bool) -> Result<Vec<Mailbox>> {
        let reference: ImapMailbox<'static> = ""
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list reference"))?;
        let pattern: ListMailbox<'static> = "*"
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP list pattern"))?;

        let rows = self.list(reference, pattern)?;

        let mut mailboxes: Vec<Mailbox> = rows
            .into_iter()
            .filter(is_selectable)
            .map(mailbox_from)
            .collect();

        if with_counts {
            for mailbox in &mut mailboxes {
                let mbox = parse_mailbox(&mailbox.id)?;
                let items = self.status(
                    mbox,
                    vec![StatusDataItemName::Messages, StatusDataItemName::Unseen].into(),
                )?;
                apply_status(mailbox, items);
            }
        }

        Ok(mailboxes)
    }

    /// Lists envelopes from `mailbox`, most recent first. `page = None`
    /// and `page_size = None` fetch the whole mailbox.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: Option<u32>,
        page_size: Option<u32>,
        with_attachment: bool,
    ) -> Result<Vec<Envelope>> {
        let mbox = parse_mailbox(mailbox)?;
        let select = self.select(mbox, ImapMailboxSelectOptions::default())?;
        let exists = select.exists.unwrap_or(0);

        let Some(window) = compute_window(exists, page, page_size) else {
            return Ok(Vec::new());
        };
        let sequence_set: SequenceSet = window
            .as_str()
            .try_into()
            .map_err(|_| anyhow!("Invalid IMAP sequence-set window `{window}`"))?;

        let data = self.fetch(
            sequence_set,
            build_item_names(with_attachment),
            ImapMessageFetchOptions::default(),
        )?;

        let envelopes = data
            .into_iter()
            .rev()
            .map(|(seq, items)| envelope_from(seq.get(), items.into_inner()))
            .collect();

        Ok(envelopes)
    }

    /// Adds, sets, or removes `flags` on a UID set in `mailbox`.
    pub fn store_flags(
        &mut self,
        mailbox: &str,
        ids: &[&str],
        flags: &[Flag],
        op: FlagOp,
    ) -> Result<()> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(ids)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();
        let kind = match op {
            FlagOp::Add => StoreType::Add,
            FlagOp::Remove => StoreType::Remove,
        };

        self.select(mbox, ImapMailboxSelectOptions::default())?;
        self.store(
            sequence_set,
            kind,
            imap_flags,
            ImapMessageStoreOptions { uid: true },
        )?;

        Ok(())
    }

    /// Fetches one message's raw RFC 5322 bytes without flipping
    /// `\Seen` (BODY.PEEK[]).
    pub fn get_message(&mut self, mailbox: &str, id: &str) -> Result<Vec<u8>> {
        let mbox = parse_mailbox(mailbox)?;
        let sequence_set = parse_uids(&[id])?;

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(vec![MessageDataItemName::BodyExt {
                section: None,
                partial: None,
                peek: true,
            }]);
        let data = self.fetch(
            sequence_set,
            item_names,
            ImapMessageFetchOptions {
                uid: true,
                ..Default::default()
            },
        )?;

        data.into_values()
            .flat_map(|items| items.into_inner().into_iter())
            .find_map(|item| match item {
                MessageDataItem::BodyExt { data, .. } => data.0.map(|d| d.as_ref().to_vec()),
                _ => None,
            })
            .ok_or_else(|| anyhow!("FETCH returned no body for the requested message"))
    }

    /// Appends `raw` to `mailbox` with `flags`, returning the appended
    /// UID (UIDPLUS APPENDUID, else a UID SEARCH on Message-ID).
    pub fn add_message(&mut self, mailbox: &str, flags: &[Flag], raw: Vec<u8>) -> Result<String> {
        let mbox = parse_mailbox(mailbox)?;
        let imap_flags: Vec<ImapFlag<'static>> = flags.iter().map(flag_from).collect();

        let (_, appenduid) = self.append(
            mbox.clone(),
            &raw,
            ImapMessageAppendOptions {
                flags: imap_flags,
                date: None,
                non_sync: false,
            },
        )?;

        if let Some((_, uid)) = appenduid {
            return Ok(uid.to_string());
        }

        // No UIDPLUS: recover the UID via SELECT + UID SEARCH on the
        // message's own Message-ID (needs one on the message).
        let message_id = MessageParser::default()
            .parse_headers(&raw)
            .and_then(|parsed| parsed.message_id().map(str::to_string))
            .filter(|id| !id.is_empty());
        let Some(message_id) = message_id else {
            bail!(
                "Cannot resolve appended UID: server lacks UIDPLUS and message has no Message-ID"
            );
        };

        self.select(mbox, ImapMailboxSelectOptions::default())?;

        let field =
            AString::try_from("Message-ID").map_err(|_| anyhow!("Invalid IMAP search header"))?;
        let value = AString::try_from(message_id)
            .map_err(|_| anyhow!("Invalid IMAP search Message-ID value"))?;
        let criteria = Vec1::from(SearchKey::Header(field, value));
        let uids = self.search(criteria, ImapMessageSearchOptions { uid: true })?;

        uids.into_iter()
            .max()
            .map(|uid| uid.to_string())
            .ok_or_else(|| anyhow!("Fallback UID search returned no match"))
    }

    /// Copies a UID set from `from` to `to`.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;
        self.copy(sequence_set, target, ImapMessageCopyOptions { uid: true })?;

        Ok(())
    }

    /// Moves a UID set from `from` to `to` (RFC 6851).
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<()> {
        let source = parse_mailbox(from)?;
        let target = parse_mailbox(to)?;
        let sequence_set = parse_uids(ids)?;

        self.select(source, ImapMailboxSelectOptions::default())?;
        self.r#move(sequence_set, target, ImapMessageMoveOptions { uid: true })?;

        Ok(())
    }
}

/// One IMAP LIST row (mailbox, delimiter, attributes).
type ListRow = (
    ImapMailbox<'static>,
    Option<QuotedChar>,
    Vec<FlagNameAttribute<'static>>,
);

/// Drops `\Noselect` containers (RFC 3501 §6.3.8): they cannot hold
/// messages and would error out on any later shared-API op.
fn is_selectable(row: &ListRow) -> bool {
    !row.2.contains(&FlagNameAttribute::Noselect)
}

/// Converts one IMAP LIST row into the shared [`Mailbox`] shape.
fn mailbox_from(row: ListRow) -> Mailbox {
    let name = match row.0 {
        ImapMailbox::Inbox => "Inbox".to_string(),
        ImapMailbox::Other(other) => String::from_utf8_lossy(other.inner().as_ref()).into_owned(),
    };

    Mailbox {
        id: name.clone(),
        name,
        total: None,
        unread: None,
    }
}

/// Folds a STATUS response into the matching mailbox row.
fn apply_status(mailbox: &mut Mailbox, items: Vec<StatusDataItem>) {
    for item in items {
        match item {
            StatusDataItem::Messages(n) => mailbox.total = Some(u64::from(n)),
            StatusDataItem::Unseen(n) => mailbox.unread = Some(u64::from(n)),
            _ => {}
        }
    }
}

/// FETCH item-name list: UID + FLAGS + ENVELOPE + RFC822.SIZE, plus
/// BODYSTRUCTURE when `with_attachment` is set.
fn build_item_names(with_attachment: bool) -> MacroOrMessageDataItemNames<'static> {
    let mut names = vec![
        MessageDataItemName::Uid,
        MessageDataItemName::Flags,
        MessageDataItemName::Envelope,
        MessageDataItemName::Rfc822Size,
    ];
    if with_attachment {
        names.push(MessageDataItemName::BodyStructure);
    }
    MacroOrMessageDataItemNames::MessageDataItemNames(names)
}

/// Sequence-set string for `(page, page_size)` against `exists`, or
/// `None` for an empty window. Page 1 is the most recent window.
fn compute_window(exists: u32, page: Option<u32>, page_size: Option<u32>) -> Option<String> {
    if exists == 0 {
        return None;
    }
    let page = page.unwrap_or(1).max(1);
    let Some(size) = page_size else {
        return Some("1:*".to_string());
    };
    if size == 0 {
        return None;
    }
    let skip = (page - 1).saturating_mul(size);
    if skip >= exists {
        return None;
    }
    let end = exists - skip;
    let start = end.saturating_sub(size - 1).max(1);
    Some(format!("{start}:{end}"))
}

/// Folds one FETCH row into a shared [`Envelope`].
fn envelope_from(seq: u32, items: Vec<MessageDataItem<'static>>) -> Envelope {
    let mut id = String::new();
    let mut message_id: Option<String> = None;
    let mut flags = BTreeSet::new();
    let mut subject = String::new();
    let mut from = Vec::new();
    let mut to = Vec::new();
    let mut date: Option<DateTime<FixedOffset>> = None;
    let mut size: u64 = 0;
    let mut has_attachment: Option<bool> = None;

    for item in items {
        match item {
            MessageDataItem::Uid(uid) => id = uid.get().to_string(),
            MessageDataItem::Flags(fs) => {
                flags = fs.into_iter().filter_map(flag_from_fetch).collect();
            }
            MessageDataItem::Envelope(env) => {
                if let Some(s) = env.subject.into_option() {
                    subject = decode_mime_bytes(s.as_ref());
                }
                if let Some(d) = env.date.into_option() {
                    date = parse_rfc2822_date(&bytes_to_string(d.as_ref()));
                }
                if let Some(m) = env.message_id.into_option() {
                    message_id = normalize_message_id(&bytes_to_string(m.as_ref()));
                }
                from = env.from.iter().map(address_from).collect();
                to = env.to.iter().map(address_from).collect();
            }
            MessageDataItem::Rfc822Size(n) => size = u64::from(n),
            MessageDataItem::BodyStructure(structure) => {
                has_attachment = Some(body_structure_has_attachment(&structure));
            }
            _ => {}
        }
    }

    if id.is_empty() {
        id = seq.to_string();
    }

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

fn flag_from_fetch(fetch: FlagFetch<'_>) -> Option<Flag> {
    let FlagFetch::Flag(flag) = fetch else {
        return None;
    };
    Some(Flag::from_raw(flag.to_string()))
}

fn address_from(addr: &ImapAddress<'_>) -> Address {
    let name = addr
        .name
        .0
        .as_ref()
        .map(|s| decode_mime_bytes(s.as_ref()))
        .filter(|s| !s.is_empty());

    let mailbox = addr
        .mailbox
        .0
        .as_ref()
        .map(|s| bytes_to_string(s.as_ref()))
        .unwrap_or_default();
    let host = addr
        .host
        .0
        .as_ref()
        .map(|s| bytes_to_string(s.as_ref()))
        .unwrap_or_default();

    let email = if mailbox.is_empty() {
        host
    } else if host.is_empty() {
        mailbox
    } else {
        format!("{mailbox}@{host}")
    };

    Address { name, email }
}

fn body_structure_has_attachment(structure: &BodyStructure<'_>) -> bool {
    match structure {
        BodyStructure::Single { extension_data, .. } => extension_data
            .as_ref()
            .and_then(|ext| ext.tail.as_ref())
            .and_then(|disposition| disposition.disposition.as_ref())
            .map(|(kind, _)| kind.as_ref().eq_ignore_ascii_case(b"attachment"))
            .unwrap_or(false),
        BodyStructure::Multi { bodies, .. } => {
            bodies.as_ref().iter().any(body_structure_has_attachment)
        }
    }
}

/// Maps a shared [`Flag`] to its IMAP wire counterpart. IANA flags
/// become the matching system flag; custom keywords pass through as
/// Keyword atoms, with a sanitised fallback for non-atom-safe input.
fn flag_from(flag: &Flag) -> ImapFlag<'static> {
    match flag.iana() {
        Some(IanaFlag::Seen) => ImapFlag::Seen,
        Some(IanaFlag::Answered) => ImapFlag::Answered,
        Some(IanaFlag::Flagged) => ImapFlag::Flagged,
        Some(IanaFlag::Draft) => ImapFlag::Draft,
        Some(IanaFlag::Deleted) => ImapFlag::Deleted,
        Some(_) => ImapFlag::keyword(
            Atom::try_from(String::from(flag.raw()))
                .expect("canonical IANA keyword is a valid IMAP atom"),
        ),
        None => match Atom::try_from(String::from(flag.raw())) {
            Ok(atom) => ImapFlag::keyword(atom),
            Err(_) => ImapFlag::keyword(
                Atom::try_from(sanitise_atom(flag.raw()))
                    .expect("sanitised atom contains only atom-safe ASCII"),
            ),
        },
    }
}

/// Replaces every non-atom-safe byte with `_` so a keyword with spaces,
/// controls or `()<>{}` survives IMAP STORE.
fn sanitise_atom(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii()
                && !c.is_control()
                && !matches!(
                    c,
                    ' ' | '(' | ')' | '{' | '%' | '*' | '"' | '\\' | ']' | '\x7f'
                )
            {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Parses a shared mailbox name into an IMAP Mailbox token.
fn parse_mailbox(name: &str) -> Result<ImapMailbox<'static>> {
    String::from(name)
        .try_into()
        .map_err(|_| anyhow!("Invalid IMAP mailbox `{name}`"))
}

/// Parses stringified UIDs into an IMAP [`SequenceSet`].
fn parse_uids(ids: &[&str]) -> Result<SequenceSet> {
    if ids.is_empty() {
        bail!("Empty UID set");
    }

    let uids: Vec<NonZeroU32> = ids
        .iter()
        .map(|s| {
            s.parse::<NonZeroU32>()
                .map_err(|_| anyhow!("Invalid message UID `{s}`"))
        })
        .collect::<Result<_>>()?;

    SequenceSet::try_from(uids).map_err(|_| anyhow!("Invalid UID set"))
}

fn parse_rfc2822_date(raw: &str) -> Option<DateTime<FixedOffset>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    DateTime::parse_from_rfc2822(trimmed).ok()
}

fn bytes_to_string(bytes: &[u8]) -> String {
    from_utf8(bytes).map(str::to_string).unwrap_or_else(|_| {
        let mut out = String::with_capacity(bytes.len());
        for b in bytes {
            out.push(*b as char);
        }
        out
    })
}

/// Decodes RFC 2047 MIME-encoded words from IMAP ENVELOPE strings;
/// falls back to [`bytes_to_string`] on malformed input.
fn decode_mime_bytes(bytes: &[u8]) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    decoder
        .decode(bytes)
        .unwrap_or_else(|_| bytes_to_string(bytes))
}
