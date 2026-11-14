//! # Mailbox
//!
//! A mailbox, as `mailbox list` prints one.

use serde::Deserialize;

/// A mailbox (a.k.a. folder).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mailbox {
    /// Backend-specific identifier, the value `-m` takes back.
    pub id: String,
    /// Human-readable mailbox name.
    pub name: String,
    /// The role the backend or `mailbox.alias.<role>` gives it (`inbox`,
    /// `sent`, `drafts`, `trash`...), `None` for an ordinary one.
    #[serde(default)]
    pub role: Option<String>,
}
