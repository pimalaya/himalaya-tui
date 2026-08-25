//! Best-effort special-use mailbox discovery for the wizard.
//!
//! The discovered roles are folded into the generated account as
//! `mailbox.alias.*`, which the himalaya CLI addresses mailboxes by.
//! The TUI resolves a mailbox by name against the live listing and
//! needs none of it, but one file backs both binaries.
//!
//! Discovery reuses the connection opened for the account test and
//! never fails the wizard: an inconclusive listing yields fewer aliases.

use std::collections::HashMap;

/// The IMAP inbox alias.
///
/// `INBOX` is a reserved, case-insensitive mailbox name every IMAP
/// server exposes (RFC 3501), so it is always safe to pin. The other
/// special-use roles are deferred until io-imap can issue LIST
/// `RETURN (SPECIAL-USE)` (RFC 6154): imap-codec has no support yet
/// (duesee/imap-codec#350), and a plain LIST advertises the attributes
/// only on some servers.
#[cfg(feature = "imap")]
pub fn imap_aliases() -> HashMap<String, String> {
    HashMap::from([("inbox".to_string(), "INBOX".to_string())])
}

/// Maps the JMAP mailbox roles (RFC 8621, authoritative) to alias keys,
/// keyed by the opaque mailbox id.
///
/// Best-effort: a failed `Mailbox/get` (or a role-less mailbox) is
/// simply skipped, since the connection was already validated by the
/// caller.
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
