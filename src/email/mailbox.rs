//! # Mailbox
//!
//! The mailbox shape shared across all protocols.

use serde::{Deserialize, Serialize};

/// A mailbox (a.k.a. folder).
///
/// Strict least-common-denominator: protocol-specific data (IMAP
/// delimiter and SPECIAL-USE attributes, JMAP role and rights, Maildir
/// path, …) is intentionally absent.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Mailbox {
    /// Backend-specific identifier: a JMAP id, the name elsewhere.
    pub id: String,
    /// Human-readable mailbox name.
    pub name: String,
    /// Total number of messages; `None` when not asked or not cheap.
    #[serde(default)]
    pub total: Option<u64>,
    /// Number of unread messages; `None` when not asked or not cheap.
    #[serde(default)]
    pub unread: Option<u64>,
}
