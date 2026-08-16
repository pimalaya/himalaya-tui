# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release of the ratatui-based three-pane TUI (mailboxes, envelopes, message or compose).
- `clap`-driven CLI: `-a/--account` to pick a configured account, positional `[EMAIL]` to seed the fallback wizard instead, `-c/--config`, `--no-config`, `--from`, `--from-name`.

  The two are mutually exclusive: `-a` addresses the configuration and errors on an unknown name, listing the accounts the file does hold, while the positional argument skips the lookup and opens a throwaway account.
- Configuration file is shared with the [`himalaya`](https://github.com/pimalaya/himalaya) CLI: same `[accounts.<name>]` blocks load on both binaries; TUI-only and CLI-only fields coexist.
- In-app composer based on [edtui](https://crates.io/crates/edtui) with `Alt-e` system-editor handoff; drafts are written in [MML](https://github.com/pimalaya/mml).
- Fallback wizard building an in-memory account when the configuration resolves none, discovering through PACC, Thunderbird Autoconfiguration (ISP, ISP-fallback, ISPDB) and RFC 6186 SRV.

  It writes nothing and proposes no configuration entry, the file being authored by the himalaya CLI. A missing configuration file or a missing default account warns before the prompts start; `--no-config` does not, having asked for the wizard. Probes resolve DNS through the host's own resolver, overridable with `HIMALAYA_DNS_RESOLVER` and falling back to Cloudflare's `1.1.1.1` over TCP.
- Backend support: IMAP, JMAP, SMTP, Maildir and m2dir, each over its own `io-*` crate (io-imap, io-jmap, io-smtp, io-maildir, io-m2dir).

  Every `server` field takes a full `scheme://` URL or a bare authority carrying an optional port, and rejects a scheme the protocol does not speak. The SMTP transport connects on the first send rather than at startup.
- SASL mechanisms: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256.
- Color themes: built-in presets (`default`, `dracula-dark`, `one-light`, `tokyo-night`) plus per-field `[theme.*]` overrides in the TOML config (`fg`, `bg`, `mod`).
- Pre-authenticated `unix://` IMAP and SMTP servers, for a local socket proxy such as sirup: no SASL is negotiated over the socket.
- Mailbox names are resolved to their backend-native id before dispatch, so the composer's `Drafts` target lands on a JMAP account too.
- `himalaya-tui completions <shell>` and `himalaya-tui manuals <dir>` auxiliary subcommands.
- 60-second idle ping against the active storage backend when the user is inactive, so long reading sessions do not lose their connection to server-side inactivity timeouts.
