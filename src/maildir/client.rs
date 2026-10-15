//! # Maildir client
//!
//! Himalaya TUI wrapper around [`io_maildir::client::MaildirClient`],
//! built from the per-account [`MaildirConfig`] block and handed to the
//! shared cross-protocol client.

use std::{
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

use anyhow::Result;
use io_maildir::{client::MaildirClient as Inner, maildir::Maildir};

use crate::config::MaildirConfig;

/// Live Maildir client wrapping io_maildir with the configured root.
pub struct MaildirClient {
    inner: Inner,
    /// Filesystem root of the configured maildir, kept so commands can
    /// join per-mailbox sub-paths without the original [`MaildirConfig`].
    pub root: PathBuf,
}

impl MaildirClient {
    /// Builds a [`MaildirClient`] rooted at the configured maildir path.
    pub fn new(config: MaildirConfig) -> Self {
        let root = config.root.clone();
        let inner = Inner::new(root.to_string_lossy().into_owned());
        Self { inner, root }
    }

    /// Resolves a mailbox argument into a loaded [`Maildir`].
    ///
    /// io-maildir resolves every logical name relative to the store root,
    /// so an absolute path is first reduced to its root-relative name,
    /// the empty one when it is the root itself. Loading validates the
    /// `cur`, `new` and `tmp` markers.
    pub fn resolve_maildir(&self, path: &Path) -> Result<Maildir> {
        let name = path.strip_prefix(&self.root).unwrap_or(path);
        Ok(self.load_maildir(name.to_string_lossy().into_owned())?)
    }
}

impl Deref for MaildirClient {
    type Target = Inner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for MaildirClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
