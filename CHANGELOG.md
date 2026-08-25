# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Initial public release of the ratatui-based three-pane TUI (mailboxes, envelopes, message or compose).
- `clap`-driven CLI: `-a/--account` to pick a configured account, positional `[EMAIL]` to answer the wizard's first prompt instead, `-c/--config`, `--no-config`, `--from`, `--from-name`. `--help` closes on the shared Pimalaya footer, pointing at the bug tracker and the sponsoring page.

  The two are mutually exclusive: `-a` addresses the configuration and errors on an unknown name, listing the accounts the file does hold, while the positional argument skips the lookup and configures what it names.
- Configuration file is shared with the [`himalaya`](https://github.com/pimalaya/himalaya) CLI: same `[accounts.<name>]` blocks load on both binaries; TUI-only and CLI-only fields coexist. The account identity reads under either spelling, `from`/`from-name` here and `email`/`display-name` in the CLI, and is written back under the CLI's, the wizard filing into the file the CLI authors.
- `signature` and `signature-delim`, at the global and account level: the composer appends the signature introduced by the delimiter, which defaults to the RFC 3676 §4.3 `"-- \n"` and is written verbatim. `signature` is the signature alone, assembled the same way as in the himalaya CLI, so one value reads the same under both binaries.
- In-app composer based on [edtui](https://crates.io/crates/edtui) with `Alt-e` system-editor handoff; drafts are written in [MML](https://github.com/pimalaya/mml).
- Setup wizard running when the configuration resolves no account: the himalaya CLI's own, prompt for prompt. One prompt takes an email address, a server URL or a local folder path; io-pim-discovery searches every service reachable from it in parallel (provider rules, PACC, Thunderbird Autoconfiguration, RFC 6186 SRV, RFC 8620 JMAP resolve) and each one that answered becomes a selectable configuration; the chosen service then prompts the authentication it advertised, the IMAP mechanisms narrowed by a live CAPABILITY probe, and every connection is tested before it is accepted. Secrets go through the OS keyring and OAuth 2.0 broker pickers rather than being stored raw.

  On the way out it offers to write the `[accounts.<name>]` block to the configuration file, or to append it to the one already there, discovered `mailbox.alias.*` entries included. Declining prints nothing, where the CLI prints the document: the account is opened either way, for the session alone when nothing was written.

  A missing configuration file is met with a welcome naming the path and the offer to generate an account; a file carrying no default account warns before the prompts start. `--no-config` and the positional argument skip both, having asked for the wizard. Probes resolve DNS through the host's own resolver, overridable with `HIMALAYA_DNS_RESOLVER` and falling back to Cloudflare's `1.1.1.1` over TCP.
- Backend support: IMAP, JMAP, SMTP, Maildir and m2dir, each over its own `io-*` crate (io-imap, io-jmap, io-smtp, io-maildir, io-m2dir).

  Every `server` field takes a full `scheme://` URL or a bare authority carrying an optional port, and rejects a scheme the protocol does not speak. The SMTP transport connects on the first send rather than at startup. Maildir flags carry custom keywords, read from the `dovecot-keywords` sidecar and from the keywords header.
- SASL mechanisms: anonymous, login, plain, oauthbearer, xoauth2, scram-sha-256.
- Color themes: built-in presets (`default`, `dracula-dark`, `one-light`, `tokyo-night`) plus per-field `[theme.*]` overrides in the TOML config (`fg`, `bg`, `mod`).
- Pre-authenticated `unix://` IMAP and SMTP servers, for a local socket proxy such as sirup: no SASL is negotiated over the socket.
- Mailbox names are resolved to their backend-native id before dispatch, so the composer's `Drafts` target lands on a JMAP account too.
- `himalaya-tui completions <shell>` and `himalaya-tui manuals <dir>` auxiliary subcommands.
- 60-second idle ping against the active storage backend when the user is inactive, so long reading sessions do not lose their connection to server-side inactivity timeouts.
