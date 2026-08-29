//! # Mailbox aliases
//!
//! Best-effort special-use mailbox discovery, folded into the generated
//! account as `mailbox.alias.*`.
//!
//! The TUI resolves a mailbox by name against the live listing and needs
//! none of it, but the himalaya CLI addresses mailboxes by alias and one
//! file backs both binaries. Discovery reuses the connection opened for
//! the account test and never fails the wizard.

use std::collections::HashMap;

/// The IMAP inbox alias.
///
/// `INBOX` is reserved on every server (RFC 3501), so it is safe to
/// pin. The other roles wait on LIST `RETURN (SPECIAL-USE)` (RFC 6154),
/// which imap-codec does not support (duesee/imap-codec#350).
#[cfg(feature = "imap")]
pub fn imap_aliases() -> HashMap<String, String> {
    HashMap::from([("inbox".to_string(), "INBOX".to_string())])
}

/// The JMAP mailbox roles (RFC 8621) as alias keys, by mailbox id.
///
/// Best-effort: a failed `Mailbox/get`, or a role-less mailbox, is
/// simply skipped, the connection having been validated by the caller.
#[cfg(feature = "jmap")]
pub fn jmap_aliases(client: &mut crate::jmap::client::JmapClient) -> HashMap<String, String> {
    use io_jmap::rfc8621::mailbox::{JmapMailboxRole, get::JmapMailboxGetOptions};

    let Ok(output) = client.mailbox_get(JmapMailboxGetOptions {
        ids: None,
        properties: None,
    }) else {
        return HashMap::new();
    };

    let mut aliases = HashMap::new();

    for mailbox in output.mailboxes {
        let Some(id) = mailbox.id else {
            continue;
        };

        let key = match mailbox.role {
            Some(JmapMailboxRole::Inbox) => "inbox",
            Some(JmapMailboxRole::Archive) => "archive",
            Some(JmapMailboxRole::Drafts) => "drafts",
            Some(JmapMailboxRole::Flagged) => "flagged",
            Some(JmapMailboxRole::Important) => "important",
            Some(JmapMailboxRole::Junk) => "junk",
            Some(JmapMailboxRole::Sent) => "sent",
            Some(JmapMailboxRole::Trash) => "trash",
            _ => continue,
        };

        aliases.insert(key.to_string(), id);
    }

    aliases
}
