---
cairn: tasks
change: the-tui-runs-on-himalaya
---

# Tasks

## Before

- [x] himalaya's side of the change landed and pushed: d70fb8d1.
- [ ] Measure today's release binary size, for the comparison after.

## Linking

- [x] Cargo.toml: `himalaya` under `[dependencies]` and its git revision under `[patch.crates-io]`; repeat himalaya's own patch entries at that revision (pimalaya-cli, mime-rs); no comment in either section.
- [x] Features forward to himalaya's (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`, the TLS providers, `vendored`); default without `sieve`.
- [x] pimalaya-cli and pimalaya-config to 0.4, mime-rs to 0.1 at himalaya's revision; mml checked against that mime-rs.
- [x] `cargo tree -d`: no second pimalaya-cli, pimalaya-config or mime-rs.

## The client module

- [x] src/client.rs: the process session, the argv runner over a `no_binary_name` wrapper of `himalaya::cli::Command`, the TUI's `Printer` (values, events, raw bytes), the three outcomes, `claim_stdin` at startup.
- [x] Decoded structs for the `mailbox list`, `envelope list`, copy, move, flag, send and add answers, mirroring `himalaya json-schema`.
- [x] Each `model.client` call moved to its command (src/tui/update.rs:59, :857, :888, :915, :933, :948, :970, :993, :1020, :1046, :1086).
- [x] The views and `Model` read the decoded structs where they read src/email's types.

## Behaviour

- [x] Paging from `total` and `nextPage`; an empty next page keeps the page shown and marks it the last.
- [x] Drafts to `-m drafts`; a sent copy through `message.send.save-copy`; queued writes said so.
- [x] Startup commands before the screen is taken; a client-building command with the terminal handed back; `Session::keep_alive` on the idle tick.

## Configuration

- [x] The account through `himalaya::cli::resolve_account`; identity and signature from himalaya's merged account; the TUI keys struct.
- [x] `-c` one path per flag (pimalaya-cli 0.4's shared flag); the missing-configuration error before the screen.
- [x] Remove the positional seed, `--no-config` and the wizard.

## Removal

- [x] Delete src/shared, src/imap, src/jmap, src/maildir, src/smtp, src/wizard and the backend half of src/config.rs; src/email stays, as decoders of himalaya's JSON.
- [x] Drop io-imap, the `nom` pin, io-jmap, io-maildir, io-smtp, io-sasl, io-pim-discovery, pimalaya-stream, base64, url, uds_windows; then shellexpand, secrecy and rfc2047-decoder if unused.
- [x] src/main.rs header: the TUI as a frontend of himalaya, running its commands over one session.

## Checks

- [x] Tests through the client module on a scratch Maildir: list mailboxes, page with `total`, read raw, copy to a name holding a space, a partial move decoded with its `errors`, add and remove a flag, save a draft byte for byte, a failed command dropping the client, a missing configuration. Not tested: the stdin refusal (`claim_stdin` is process-global, and no argv the TUI builds reads stdin).
- [x] Unit tests of the paging rule with and without a total. The argv builder has no unit test of its own: every argv goes through the Maildir tests above, a name holding a space among them.
- [x] Clippy `-D warnings` on every feature set (default, `--no-default-features` with one backend at a time, `--all-features`).
- [ ] Live checks on the Fastmail (JMAP), Google (Gmail) and Microsoft (Graph) test accounts: read-only listing and reading, writes in a scratch mailbox only.
- [ ] Release binary size compared with the one measured before.

## Docs and landing

- [x] README: backends, `-c`, no wizard until phase 2.
- [x] CHANGELOG under [Unreleased]: Breaking `-c` one path per flag; Removed the wizard, the seed and `--no-config`; Added Gmail, Microsoft Graph, mbox and pimdir; Changed drafts by role and the sent copy.
- [x] Fold the delta into configuration.md and envelopes.md, write the log entry, set `landed`.
