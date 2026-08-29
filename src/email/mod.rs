//! # Email vocabulary
//!
//! The shared cross-protocol email types the interface renders, inlined
//! from the retired io-email crate.
//!
//! Strict least-common-denominator: the per-backend adapters producing
//! these shapes live in each protocol module's backend submodule and keep
//! their own vocabulary behind it.

pub mod address;
pub mod envelope;
pub mod flag;
pub mod mailbox;
