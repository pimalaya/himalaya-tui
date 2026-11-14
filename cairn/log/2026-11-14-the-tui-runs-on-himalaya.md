---
cairn: log
change: the-tui-runs-on-himalaya
landed: 2026-11-14
---

# The TUI runs on himalaya

Phase 1 of the plan of 2026-11-14: the TUI drops its own backend layer and runs himalaya's commands in-process. Capabilities **client** (new), **configuration** and **envelopes** moved.

## What landed

- **Cargo.toml**: `himalaya` 2.2 with every backend feature forwarded (`imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox`, `pimdir`, the TLS providers, `vendored`; no `sieve`). `[patch.crates-io]` repeats himalaya's own entries (pimalaya-cli 0be1b1f, mime-rs 2591e7f, io-imap, io-jmap, io-maildir, io-gmail), a dependency's patches never reaching the root build; `cargo tree` shows one pimalaya-cli. pimalaya-cli and pimalaya-config moved to 0.4, mime-rs to 0.1. himalaya is pinned to d70fb8d1, the commit landing its side.
- **src/client.rs** (`MailClient`): one himalaya `Session` for the process, its paths leaked once since the session borrows them for the run. A command is an argv parsed by a `no_binary_name` wrapper over `himalaya::cli::Command`, as the MCP server's `McpLine`, run with `Command::execute`. The printer (`Captured`) keeps the JSON value and the raw bytes of `message read --raw`, which `CapturePrinter` would base64. A `PartialOutcome` error is a result. Answers decode into the TUI's own structs (src/email, now camelCase decoders of the documented JSON, `Mailbox` gaining `role`).
- **Messages through a file**: `message send` and `message add` read a private `NamedTempFile`. himalaya's `MessageArg` turns the literal `\n` and `\r` sequences of an inline message into line endings, so an inline draft quoting `printf("\n")` came back altered: `a_draft_is_saved_byte_for_byte_to_the_drafts_alias` fails with the message inline and passes through the file.
- **Actions**: drafts go to `-m drafts` (alias, else role), sends copy through `message.send.save-copy`, a queued write says so, a read uses `--raw` without `--seen` (flags untouched, as the old `BODY.PEEK[]`). Copy, move and flag report `Moved n message(s)` and the first item error.
- **Paging**: `total` from `envelope list` gives `(page n/p)`; without one, `(page n)` and the next page while `nextPage` is set. A page coming back empty past the first is recorded (`envelope_end`) and the page before it reloaded and marked the last (`has_next_page`, a pure function now tested).
- **Connection**: the startup commands run in `try_into_tui_model`, before `app::run` takes the terminal. A deferred action running while `MailClient::is_connected` is false hands the terminal back around it, as the system editor does. The idle tick calls `Session::keep_alive` where it pinged.
- **Configuration**: the account through `himalaya::cli::resolve_account`, identity and signature from himalaya's merged `Account` (`signature_delim()` defaulting to `"-- \n"`). src/config.rs keeps `keybinds`, `contact-command`, `contact-complete-key` and the theme, loaded through pimalaya-config 0.4's `TomlConfig` with himalaya's paths and sample URL. `-c` is pimalaya-cli's `ConfigFlags` (one path per flag, no `HIMALAYA_CONFIG`), `-a` its `AccountFlag`, and `-b/--backend` is new. `claim_stdin` and `SimdCpu::install_detected` run first in `main`.
- **Removed**: src/shared, src/imap, src/jmap, src/maildir, src/smtp, src/wizard and the backend half of src/config.rs (4,480 lines in the removed modules, 5,698 deleted and 385 added under src overall); the positional seed and `--no-config`; io-imap and its `nom` pin, io-jmap, io-maildir, io-smtp, io-sasl, io-pim-discovery, pimalaya-stream, base64, url, uds_windows, rfc2047-decoder, secrecy, shellexpand.
- **Docs**: config.sample.toml keeps the TUI keys and the theme and points at himalaya's sample for accounts; README and CHANGELOG follow.

## Checks

- `cargo test`: 34 tests, among them src/client.rs against a scratch Maildir driven through himalaya (mailbox list, a page with `total` and `nextPage`, a raw read, a copy to a mailbox whose name holds a space, a partial move naming the missing id, a flag added then removed, a draft saved byte for byte to the drafts alias, a failed command dropping the client, a missing configuration naming its path) and the config tests reading the shipped sample through the TUI's struct and himalaya's.
- clippy `-D warnings` clean on the default set, `--all-features`, `native-tls,jmap` and `rustls-ring` with each backend alone; `cargo deny check` passes (duplicate-version warnings only).
- `himalaya-tui -c <missing>` fails before drawing, naming the path and himalaya's sample.

## Not done

- Live checks on the Fastmail, Google and Microsoft test accounts, and the release binary size against the old one.
