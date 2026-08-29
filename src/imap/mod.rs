//! # IMAP
//!
//! IMAP backend: a thin wrapper around io-imap's high-level session,
//! plus the adapter mapping its wire types onto the shared
//! [`crate::email`] domain types.

pub mod backend;
pub mod client;
