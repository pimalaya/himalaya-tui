---
cairn: log
change: shared-config-parity
landed: 2026-08-16
---

# Imported the CLI options the shared configuration blocks were missing

`imap.sort.fallback` in a working himalaya CLI file aborted the TUI's startup with an unknown-field error, the per-backend blocks carrying `deny_unknown_fields`. Six options were in that position: `imap.alpn`, `imap.sasl-ir`, `imap.sort.fallback`, `smtp.alpn`, `jmap.alpn`, `jmap.identity-id` and `jmap.drafts-mailbox-id`.

All of them now deserialize, and all but one do something. The three `alpn` lists replace the token each client hardcoded into `tls.rustls.alpn`, reaching it through the new `TlsConfig::into_tls(alpn)` which supersedes the infallible `TryFrom<TlsConfig> for Tls`. `imap.sasl-ir` reaches `ImapSessionOpenOptions::sasl_ir`. `jmap.identity-id` and `jmap.drafts-mailbox-id` are carried on `JmapClient` and preferred by `resolve_identity_id` and `resolve_drafts_mailbox_id`, which keep resolving from the live session when unset. `imap.sort.fallback` is accepted and inert: the TUI paginates a sequence-set window and reverses it rather than issuing `SORT`.

The capability file cairn/spec/configuration.md is new and holds the delta: a CLI-valid file loads in the TUI, ALPN comes from the configuration, and the JMAP send targets are configurable. Two tests pin the shared vocabulary and the ALPN defaults, and the CLI's own config.sample.toml was confirmed to load under the TUI's model. config.sample.toml documents each new option.
