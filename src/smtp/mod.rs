//! # SMTP
//!
//! SMTP backend: a thin wrapper around
//! [`io_smtp::client::SmtpClientStd`], plus the send adapter for the
//! shared cross-protocol client.

pub mod backend;
pub mod client;
