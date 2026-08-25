---
cairn: log
change: bump-deps-maildir-keywords
landed: 2026-08-25
---

# Bumped every dependency, and handed Maildir flag resolution to io-maildir

io-imap went 0.5 to 0.6 and io-maildir 0.2 to 0.3, both majors the himalaya CLI already carries; pimalaya-cli moved 0.2.1 to 0.2.4 and mail-parser, edtui, log and the ratatui tree took their patch releases through the lockfile. io-imap 0.6 breaks on the mailbox watch alone, which this binary does not use, so the bump cost nothing there.

io-maildir 0.3 broke `read_entries`, which now takes the Maildir the entries were listed from so it can resolve each entry's custom keywords through the `dovecot-keywords` sidecar and the keywords header. `list_envelopes` passes it, and `envelope_from_entry` reads `MaildirFullEntry::flags` rather than parsing the filename's info section itself: `parse_filename_flags` and `flag_from_char` are gone, replaced by `flag_from_maildir`, the inverse of the `flag_to_maildir` the adapter already used on the way out and the same mapping the CLI carries. Envelopes now show custom keywords where they used to show the six IANA letters and nothing else.

The release also fixed `add_flags` and `set_flags` renaming nothing for an entry in `new/`, which is a flag write reported as successful and lost; the TUI inherits that with no change of its own.

No capability file moved. The spec covers configuration, and how a Maildir names a keyword is io-maildir's business rather than a requirement this binary states.
