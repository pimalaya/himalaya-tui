//! # JMAP
//!
//! JMAP backend: a thin wrapper around io_jmap's high-level client, plus
//! the adapter lowering its responses into the shared cross-protocol
//! email domain types.

pub mod backend;
pub mod client;
