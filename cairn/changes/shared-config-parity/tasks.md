---
cairn: tasks
change: shared-config-parity
---

- [x] Diff the TUI's configuration structs against the CLI's, block by block, listing every option a `deny_unknown_fields` block would reject.
- [x] Add `alpn` to the IMAP, SMTP and JMAP blocks with the per-protocol defaults, and feed it to the clients through `TlsConfig::into_tls`.
- [x] Add `imap.sasl-ir` and pass it to `ImapSessionOpenOptions`.
- [x] Add `imap.sort` and document it as read by the CLI only.
- [x] Add `jmap.identity-id` and `jmap.drafts-mailbox-id`, preferring them over session resolution when set.
- [x] Update the wizard's config literals for the new fields.
- [x] Document every new option in config.sample.toml.
- [x] Cover the shared vocabulary with tests, and confirm the CLI's own config.sample.toml loads under the TUI's model.
- [x] Re-check every backend feature combination, then run fmt, clippy and the test suite.
