---
cairn: log
change: maildir-relative-names
landed: 2026-10-15
---

# Maildir mailboxes are named relative to the root

Raised in #22. A Maildir's name was its last path segment (io-maildir `Maildir::name`), so `Archive` and `[Gmail]/Archive` showed as two identical rows, and Maildir++ folders kept their leading dot. `MaildirClient::mailbox_name` in src/maildir/client.rs now names a Maildir through `MaildirStore::relative`, both layouts giving `[Gmail]/Archive`, the root keeping its folder name, as the CLI does. Ids stay absolute paths.

No capability moved.
