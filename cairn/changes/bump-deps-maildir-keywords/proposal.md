---
cairn: change
id: bump-deps-maildir-keywords
status: landed
created: 2026-08-25
---

# Bring every dependency to its latest release, and let io-maildir resolve the flags

The last bump was nine days ago. Since then io-imap released 0.6 and io-maildir 0.3, both breaking, and a dozen crates moved by a patch release. The himalaya CLI is already on both majors, and the two binaries reading one configuration file is worth little if they read one Maildir differently.

io-maildir 0.3 is where the behaviour is. `read_entries` now takes the Maildir the entries were listed from and resolves each entry's custom keywords through the `dovecot-keywords` sidecar and the keywords header, which is why the signature changed; `MaildirFullEntry::flags` hands back the result. The release also fixed `add_flags` and `set_flags` silently discarding the flags of an entry in `new/`, so marking a message seen in the TUI now actually marks it.

## What changes

io-imap 0.5 to 0.6 and io-maildir 0.2 to 0.3 in Cargo.toml, everything else through the lockfile: pimalaya-cli 0.2.1 to 0.2.4, mail-parser, edtui, log, ratatui's tree and the usual transitive churn. io-imap 0.6's breaking surface is the mailbox watch, which this binary does not use, so nothing there moves.

The Maildir adapter drops `parse_filename_flags` and `flag_from_char`, which read the six IANA letters straight off the filename and could not have seen a keyword, for `MaildirFullEntry::flags` mapped through a `flag_from_maildir` matching the CLI's. Envelopes carry custom keywords from here on, and the mapping the adapter already had in the other direction, `flag_to_maildir`, gains its inverse rather than a second spelling of one.

mime-meta-language stays pinned to the mml git repository: the composer still targets an API 1.1.1 does not carry.
