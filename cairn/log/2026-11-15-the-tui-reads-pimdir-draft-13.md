---
cairn: log
change: the-tui-reads-pimdir-draft-13
landed: 2026-11-15
---

# The TUI reads pimdir draft-13

Line X17 of the CLI alignment of 2026-11-15.

## What landed

- Pins: himalaya 0154fd1f (io-pimdir 0.14.0, pimdir storage-draft-13), pimalaya-cli 0.6.0, pimalaya-config 0.5.2; mml stays at 563dd51, himalaya not depending on it. The Cargo.lock moves himalaya's source, io-pimdir and pimalaya-cli alone: the re-resolve also moved windows-sys 0.59 to 0.61 in eight entries and getrandom in one, which were put back.
- src/email/envelope.rs: `subject` is optional, himalaya printing `null` for a message without `Subject:`; src/tui/view.rs shows an empty cell; src/client.rs tests a Maildir message without a subject.

## Capabilities moved

- [client](../spec/client.md): *A pimdir account opens the store himalaya reads*, added.
- [envelopes](../spec/envelopes.md): *A message without a subject is listed*, added.
