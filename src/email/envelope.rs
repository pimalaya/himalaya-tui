//! # Envelope
//!
//! The envelope shape shared across all protocols.

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

use crate::email::{address::Address, flag::Flag};

/// Enough of a message to render a list entry without fetching its body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Envelope {
    /// Backend-specific identifier: an IMAP UID, a JMAP or Maildir id.
    pub id: String,
    /// `Message-ID:` (RFC 5322 §3.6.4), `None` when missing or unsurfaced.
    ///
    /// Normalised, so it is stable across every backend that stores the
    /// message.
    #[serde(default)]
    pub message_id: Option<String>,
    /// Flags set on the message, a sorted set: wire order means nothing.
    #[serde(default)]
    pub flags: BTreeSet<Flag>,
    /// Subject header value.
    #[serde(default)]
    pub subject: String,
    /// Sender(s).
    #[serde(default)]
    pub from: Vec<Address>,
    /// Primary recipient(s).
    #[serde(default)]
    pub to: Vec<Address>,
    /// Author-claimed send time from the `Date:` header, `None` when the
    /// header is missing or unparseable.
    #[serde(default)]
    pub date: Option<DateTime<FixedOffset>>,
    /// Size of the raw RFC 5322 message in bytes.
    #[serde(default)]
    pub size: u64,
    /// Whether the message has an attachment; `None` when not requested
    /// or not detectable on the active backend.
    #[serde(default)]
    pub has_attachment: Option<bool>,
}

/// One page of a mailbox listing, with the size of the whole.
///
/// A page alone cannot say how much is behind it, and a full one is
/// indistinguishable from the end of the mailbox, so the count the
/// backend reported travels beside the envelopes.
#[derive(Clone, Debug, Default)]
pub struct EnvelopeList {
    /// The envelopes on the requested page, newest first.
    pub envelopes: Vec<Envelope>,
    /// Messages the mailbox holds, whatever the page size.
    ///
    /// Falls back to the page length on a backend that cannot report one.
    pub total: u32,
}

/// Strips the RFC 5322 `msg-id` wrappers from a raw `Message-ID:` value.
///
/// Whitespace and a single pair of angle brackets are removed, an empty
/// result becoming `None`, so that every backend's
/// [`Envelope::message_id`] is comparable byte-for-byte.
pub fn normalize_message_id(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let inner = trimmed
        .strip_prefix('<')
        .and_then(|s| s.strip_suffix('>'))
        .unwrap_or(trimmed)
        .trim();

    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}
