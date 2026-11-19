---
cairn: log
change: the-tui-follows-himalaya-0cb2ff4d
landed: 2026-11-19
---

# The TUI follows himalaya 0cb2ff4d

Prepares the first release, which waits on himalaya 3.0 and mml 2.0 (line T2 of the CLI alignment of 2026-11-15).

## What landed

- Pins: himalaya 0cb2ff4d (io-pimdir 0.17.0, pimdir storage-draft-14), mml 1b9f3f2, pimalaya-cli 0.6.1, pimalaya-config 0.6.0. The Cargo.lock moves the two git sources and the crates himalaya's bump carries (io-pimdir, io-jmap 0.10.1, io-msgraph 0.9.3, ical-rs 0.7.1, jiff 1, rusqlite 0.43, libsqlite3-sys 0.40), defmt leaving the tree.
- src/client.rs: `SendReceipt` reads the `errors` himalaya's send receipt now carries, a copy failing after the send being a result, exit 3, where it was a failure. src/tui/update.rs: the status reads `Message sent, error: <first error>` and the composer closes; test `a_sent_message_whose_copy_failed_names_the_failure` over a scripted loopback SMTP server, gated on `smtp` and `maildir`, failing on the old status (`Message sent`).
- The rest of himalaya's changes since d98528b5 leave the TUI's commands and answers as they were (see the change's proposal).

## Capabilities moved

- [client](../spec/client.md): *A draft and a sent copy follow the account*, modified (a copy failing after the send); *A pimdir account opens the store himalaya reads*, modified (storage-draft-14).
