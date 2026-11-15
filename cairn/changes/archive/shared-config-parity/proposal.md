---
cairn: change
id: shared-config-parity
status: landed
created: 2026-08-16
---

# Accept every CLI option in the blocks the TUI shares with it

The TUI and the himalaya CLI read one configuration file, but the per-backend blocks carry `deny_unknown_fields`, so an option the CLI grew and the TUI never modelled does not degrade into a warning: it aborts the load. `imap.sort.fallback = true` in a working CLI file stopped the TUI at startup with an unknown-field error, and `imap.alpn`, `imap.sasl-ir`, `smtp.alpn`, `jmap.alpn`, `jmap.identity-id` and `jmap.drafts-mailbox-id` were all one keystroke from doing the same.

Only the blocks both binaries model are affected. The top-level table and the account table already omit `deny_unknown_fields`, so CLI-only sections (`table`, `envelope`, `mailbox`, `attachment`, `account`) and CLI-only backends (`gmail`, `msgraph`, `pimdir`) were already tolerated.

## What changes

Every missing option is added, and honoured wherever the TUI has something to honour it with:

- `imap.alpn`, `smtp.alpn` and `jmap.alpn` replace the ALPN token each client hardcoded, so `tls.rustls.alpn` is fed from the config instead of from a literal.
- `imap.sasl-ir` reaches `ImapSessionOpenOptions::sasl_ir`, which is the quirk switch it exists for.
- `jmap.identity-id` and `jmap.drafts-mailbox-id` override what the send path resolved from the live session, which stays the fallback.
- `imap.sort.fallback` is accepted and documented as inert: the TUI paginates a sequence-set window and reverses it rather than issuing `SORT`, so there is no fallback for it to pick.

`TryFrom<TlsConfig> for Tls` becomes `TlsConfig::into_tls(alpn)`, mirroring the CLI: the conversion never failed, and it now takes the ALPN list its callers were patching in afterwards.
