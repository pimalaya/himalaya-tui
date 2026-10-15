---
cairn: log
change: maildir-ids-resolve-like-the-cli
landed: 2026-10-15
---

# Maildir mailbox ids resolve like the CLI

Issue #22. A Maildir mailbox is listed with its absolute path as id, and `resolve_maildir` in src/maildir/client.rs handed that path to io-maildir, which joins every name onto the store root: `<root>//<root>/Inbox`, so no mailbox opened, the startup inbox included.

`resolve_maildir` now strips the root first and loads the root-relative name, the empty one being the root itself, exactly as the CLI's does. Ids keep their shape, so they stay the CLI's.

No capability moved.
