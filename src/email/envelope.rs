//! # Envelope
//!
//! An envelope, as `envelope list` prints one.

use std::collections::BTreeSet;

use chrono::{DateTime, FixedOffset};
use serde::Deserialize;

use crate::email::{address::Address, flag::Flag};

/// Enough of a message to render a list entry without fetching its body.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Envelope {
    /// Backend-specific identifier: an IMAP UID, a JMAP or Maildir id.
    pub id: String,
    /// Flags set on the message, a sorted set: wire order means nothing.
    #[serde(default)]
    pub flags: BTreeSet<Flag>,
    /// Subject header value, `None` when the message has none.
    #[serde(default)]
    pub subject: Option<String>,
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
}
