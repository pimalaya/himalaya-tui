//! First-time-setup wizard: provider discovery, credential prompts and
//! the offer to file the result in the configuration.
//!
//! [`configure`] is the entry point and owns the welcome, the account
//! name and the save offer; [`discover`] is the half that decides what
//! the account is, from the one prompt it asks; [`search`] runs
//! io-pim-discovery's parallel discovery behind it. The per-service
//! flows ([`imap_smtp`], [`jmap`], [`local`]) prompt the authentication
//! their service advertised, through the pickers in [`secret`], and
//! validate what they built with [`check`].
//!
//! The flow is the himalaya CLI's, prompt for prompt, and so is what it
//! writes when the offer is accepted. What differs is that the account
//! is opened either way: see [`configure`].

pub mod check;
pub mod configure;
pub mod discover;
#[cfg(all(feature = "imap", feature = "smtp"))]
pub mod imap_smtp;
#[cfg(feature = "jmap")]
pub mod jmap;
#[cfg(any(feature = "maildir", feature = "m2dir"))]
pub mod local;
#[cfg(any(feature = "imap", feature = "jmap"))]
pub mod mailbox;
pub mod search;
#[cfg(any(feature = "imap", feature = "jmap"))]
pub mod secret;
