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

    /// The name a Maildir is shown and selected by: its path relative
    /// to the store root (`[Gmail]/Archive`), whatever the layout, so
    /// nested Maildirs sharing a last segment stay apart. The root
    /// itself keeps its folder name.
    pub fn mailbox_name(&self, maildir: &Maildir) -> String {
        match self.store.relative(maildir.path()) {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => maildir.name().unwrap_or_default().to_owned(),
        }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use io_maildir::maildir::Maildir;

    use super::MaildirClient;
    use crate::config::MaildirConfig;

    #[test]
    fn mailbox_name_is_relative_to_the_root() {
        let client = MaildirClient::new(MaildirConfig {
            root: PathBuf::from("/mail"),
        });

        let name = |path: &str| client.mailbox_name(&Maildir::from(path));
        assert_eq!(name("/mail/Archive"), "Archive");
        assert_eq!(name("/mail/[Gmail]/Archive"), "[Gmail]/Archive");
        assert_eq!(name("/mail"), "mail");
    }
}
