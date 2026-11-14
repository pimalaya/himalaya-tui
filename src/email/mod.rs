//! # Email vocabulary
//!
//! The mail types the interface renders, decoded from the `--json`
//! output of himalaya's commands (`himalaya json-schema`): the coupling
//! is that versioned contract, never himalaya's own Rust types.

pub mod address;
pub mod envelope;
pub mod flag;
pub mod mailbox;
