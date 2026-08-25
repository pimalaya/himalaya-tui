//! Mailbox shared across all protocols.

use serde::{Deserialize, Serialize};

/// A mailbox (a.k.a. folder).
///
/// Strict least-common-denominator shape: only fields that are
/// first-class in every protocol the interface targets (IMAP, JMAP,
/// Maildir). Protocol-specific data (IMAP delimiter and
/// SPECIAL-USE attributes, JMAP role and rights, Maildir path, …) is
/// intentionally absent.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Mailbox {
    /// Backend-specific identifier.
    ///
    /// JMAP exposes a real opaque ID; for IMAP and Maildir this is the
    /// same as [`Self::name`]. Use this when issuing follow-up
    /// commands that refer to the mailbox.
    pub id: String,

    /// Human-readable mailbox name.
    pub name: String,

    /// Total number of messages, when the caller requested counts.
    /// `None` when the backend was not asked or cannot answer cheaply.
    #[serde(default)]
    pub total: Option<u64>,

    /// Number of unread messages, when the caller requested counts.
    /// `None` when the backend was not asked or cannot answer cheaply.
    #[serde(default)]
    pub unread: Option<u64>,
}
