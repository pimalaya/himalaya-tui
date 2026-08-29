# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added the three-pane terminal interface: mailboxes, envelopes, and the message body or the composer.

  The envelope list is paged one screenful at a time, and the page count comes from the total the backend reports: `EXISTS` on IMAP, the query `total` on JMAP, the entry count on Maildir.

  The panel names its page `(page n/p)` when there is more than one, a resized terminal re-cuts the page around the selected envelope, and moving past either end loads the next or the previous one.

- Added the IMAP, JMAP, SMTP and Maildir backends, each behind its own cargo feature.

  Every `server` field takes a full `scheme://` URL or a bare authority carrying an optional port, and rejects a scheme the protocol does not speak. A `unix://` server is a pre-authenticated socket proxy such as sirup, over which no SASL is negotiated.

  The SMTP transport connects on the first send rather than at startup, and Maildir flags carry the custom keywords named by the `dovecot-keywords` sidecar or by the keywords header.

- Added the anonymous, login, plain, oauthbearer, xoauth2 and scram-sha-256 SASL mechanisms, plus basic and bearer HTTP authentication for JMAP.
- Added the setup wizard, run when the configuration resolves no account.

  It is the himalaya CLI's wizard prompt for prompt: one prompt for an email address, a parallel search of the services reachable from it, a pick list of those that answered, the authentication that service advertised, and a connection test before the account is accepted.

  On the way out it offers to write the `[accounts.<name>]` block to the configuration file, or to append it to the one already there.

  Two things differ from the CLI. `--no-config` forces the wizard on a run whose file resolves an account, and declining the offer prints nothing where the CLI prints the document to stdout: the account is opened either way, for the session alone when nothing was written.

- Added account discovery through known provider rules, PACC, Thunderbird Autoconfiguration, RFC 6186 SRV and the RFC 8620 JMAP session resolve, searched in parallel.

  DNS goes through the host's own resolver, overridable with `HIMALAYA_DNS_RESOLVER` and falling back to Cloudflare's `1.1.1.1` over TCP.

- Added the configuration file shared with the [himalaya](https://github.com/pimalaya/himalaya) CLI: the same `[accounts.<name>]` blocks load on both binaries.

  The account identity reads under either spelling, `from` and `from-name` or `email` and `display-name`, and is written under the CLI's. TUI-only fields and CLI-only sections coexist without errors.

- Added `signature` and `signature-delim`, at the global and the account level.

  `signature` is the signature alone and `signature-delim` decides the separator introducing it, defaulting to the RFC 3676 section 4.3 `"-- \n"` and written verbatim. The block is assembled as the himalaya CLI assembles it, so one value reads the same under both binaries.

- Added the in-app composer, with an `Alt-e` handoff to the system editor. Drafts are written in [MML](https://github.com/pimalaya/mml).
- Added the `default`, `dracula-dark`, `one-light` and `tokyo-night` color presets, plus per-field `[theme.*]` overrides.
- Added the `-a/--account`, `-c/--config`, `--no-config`, `--from` and `--from-name` flags, the `[EMAIL]` positional argument answering the wizard's first prompt, and the `completion` and `manual` subcommands.

  `-a` addresses the configuration and errors on an unknown name, listing the accounts the file does hold, while the positional argument skips the lookup entirely. The two are mutually exclusive.

  `--help` closes on the shared Pimalaya footer.

- Added mailbox name resolution to the backend-native id before dispatch, so the composer's `Drafts` target lands on a JMAP account too.
- Added a 60-second idle ping against the active storage backend, so a long reading session does not lose its connection to a server-side inactivity timeout.
