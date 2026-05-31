# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release of the ratatui-based three-pane TUI (mailboxes, envelopes, message or compose).
- `clap`-driven CLI: positional `[ACCOUNT]` doubling as wizard seed, `-c/--config`, `--no-config`, `--from`, `--from-name`.
- Configuration file is shared with the [`himalaya`](https://github.com/pimalaya/himalaya) CLI: same `[accounts.<name>]` blocks load on both binaries; TUI-only and CLI-only fields coexist.
- In-app composer based on [edtui](https://crates.io/crates/edtui) with `Alt-e` system-editor handoff; drafts are written in [MML](https://github.com/pimalaya/mml).
- Provider discovery wizard: PACC, Thunderbird Autoconfiguration (ISP, ISP-fallback, ISPDB), RFC 6186 SRV.
- Backend support: IMAP, JMAP, SMTP, Maildir and m2dir (via [io-email](https://github.com/pimalaya/io-email) and the matching `io-*` crates).
- SASL mechanisms: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256.
- Color themes: built-in presets (`default`, `dracula-dark`, `one-light`) plus per-field `[theme.*]` overrides in the TOML config (`fg`, `bg`, `mod`).
- `himalaya-tui completions <shell>` and `himalaya-tui manuals <dir>` auxiliary subcommands.

- Added a 60-second idle ping that issues a NOOP to every registered network backend (IMAP, SMTP) when the user is inactive, so long reading sessions do not lose their connections to server-side inactivity timeouts.
