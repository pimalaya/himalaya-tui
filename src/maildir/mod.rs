//! # Maildir
//!
//! Maildir backend: a thin wrapper around io-maildir's high-level
//! client, plus the adapter mapping its filesystem entries onto the
//! shared [`crate::email`] domain types.

pub mod backend;
pub mod client;
