//! # Wizard
//!
//! First-time setup: provider discovery, credential prompts, and the
//! offer to file the result in the configuration.
//!
//! [`configure`] is the entry point, [`discover`] decides what the
//! account is from its one prompt, and [`search`] runs io-pim-discovery
//! behind it. The per-service flows ([`imap_smtp`], [`jmap`]) prompt the
//! authentication their service advertised, and [`check`] validates it.
//!
//! The flow is the himalaya CLI's, prompt for prompt, except that the
//! account is opened whether or not it is filed: see [`configure`].

pub mod check;
pub mod configure;
pub mod discover;
#[cfg(all(feature = "imap", feature = "smtp"))]
pub mod imap_smtp;
#[cfg(feature = "jmap")]
pub mod jmap;
#[cfg(any(feature = "imap", feature = "jmap"))]
pub mod mailbox;
pub mod search;
#[cfg(any(feature = "imap", feature = "jmap"))]
pub mod secret;
