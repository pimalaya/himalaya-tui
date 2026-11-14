//! # Address
//!
//! An email address, as himalaya prints one.

use serde::Deserialize;

/// A single email address with an optional display name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Address {
    /// Display name (e.g. `Alice`), if any.
    pub name: Option<String>,
    /// Email address (e.g. `alice@example.org`).
    pub email: String,
}
