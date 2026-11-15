# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added himalaya's failure codes to a failure to open the account: `account-missing`, `account-default-missing`, `account-default-several`, `backend-not-built`. `-a default` names an account called `default`.
- Added the three-pane terminal interface: mailboxes, envelopes, and the message body or the composer.

  The envelope list is paged one screenful at a time, and the page count comes from the `total` himalaya's `envelope list` reports. A backend reporting none (Gmail, Microsoft Graph) shows `(page n)` and pages on while himalaya names a next page.

  The panel names its page `(page n/p)` when there is more than one, a resized terminal re-cuts the page around the selected envelope, and moving past either end loads the next or the previous one.

- Added every backend of the [himalaya](https://github.com/pimalaya/himalaya) CLI (IMAP, SMTP, JMAP, Gmail API, Microsoft Graph, Maildir, mbox, pimdir), each behind the cargo feature of the same name.

  The TUI links himalaya as a library, pinned by git revision, and runs himalaya's own commands in-process over one session kept for the whole run, as himalaya's REPL does: the client connects on the first command, is kept, and is built again after a command fails, the terminal handed back meanwhile so a credential command can prompt. Every option, credential and behaviour of an account is himalaya's.

- Added the configuration file shared with the himalaya CLI: the account, its backends, its identity and its signature are resolved through himalaya's own configuration, so one file reads the same in both binaries. The TUI adds `keybinds`, `contact-command`, `contact-complete-key` and `theme` under a `[tui]` table, and an account's `contact-command` under `[accounts.<name>.tui]`, tables himalaya leaves to the TUI.

  With no configuration, the TUI stops before drawing anything, naming the path it looked at and the sample to start from.

- Added `signature` and `signature-delim` support, read through himalaya: `signature` is the signature alone and `signature-delim` the separator introducing it, defaulting to the RFC 3676 section 4.3 `"-- \n"` and written verbatim.

- Added the in-app composer, with an `Alt-e` handoff to the system editor. Drafts are written in [MML](https://github.com/pimalaya/mml).

  Sending goes through himalaya's `message send`, which saves a copy where `message.send.save-copy` says. A draft is saved to the account's drafts mailbox, its `mailbox.alias.drafts` else the one the backend marks as such. A write a sync engine applies later (pimdir) says it is queued.

- Added recipient completion from a `contact-command`, the aerc way, triggered by `contact-complete-key` (`tab` by default) in the `To`, `Cc`, `Bcc` and `Reply-To` headers ([#16]).

  The command runs off the event loop and is killed after 5 seconds. One match is inserted directly, several open a list at the cursor.
- Added the `default`, `dracula-dark`, `one-light` and `tokyo-night` color presets, plus per-field `[tui.theme.*]` overrides.
- Added the `-c/--config` (repeated to merge files), `-a/--account`, `-b/--backend`, `--from`, `--from-name` and `--keybinds` flags, and the `completion` and `manual` subcommands.

  `-a` errors on an unknown name, listing the accounts the file does hold. `--help` closes on the shared Pimalaya footer.

- Added marks: `Space` marks the envelope under the cursor and moves down, across pages ([#9]). Copy, move, the flag actions and delete reach every marked envelope in one himalaya command, or the selected one when none is marked; the envelopes a command could not reach stay marked, and the status bar says `Moved 2 of 3 messages` and the first failure. `Esc` clears the marks, and so does selecting another mailbox. Marked rows take the new `[tui.theme.envelope-marked]` style.
- Added delete, from the envelope dialog or the `Delete` key ([#7]): himalaya's `message delete` moves the messages to the trash (`mailbox.alias.trash`, else the backend's trash role), and removes them for good when they already are there, after a confirmation. On an IMAP server without UIDPLUS they are flagged deleted, an expunge still owing.
- Added a keepalive after 60 idle seconds, a `NOOP` on IMAP, so a long reading session does not lose its connection to a server-side inactivity timeout.

### Changed

- **BREAKING**: Changed the TUI's own configuration keys to live under `tui` tables: `keybinds`, `contact-command`, `contact-complete-key` and `theme` under a top-level `[tui]`, an account's `contact-command` under `[accounts.<name>.tui]`. The `tui` tables refuse a key they do not know.

  The same keys left at the top level or in an account are no longer read, and himalaya refuses the file naming them. Move them under the `tui` tables, `[theme]` and `[theme.<element>]` becoming `[tui.theme]` and `[tui.theme.<element>]`:

  ```toml
  [tui]
  keybinds = "emacs"
  contact-command = "khard email --parsable %s"

  [tui.theme]
  preset = "tokyo-night"

  [accounts.work.tui]
  contact-command = ["abook", "--mutt-query", "%s"]
  ```

- Bumped ctutils to 0.4.6, rustls-webpki to 0.103.16 and webpki-root-certs to 1.0.11, fixing RUSTSEC-2026-0382, RUSTSEC-2026-0383 and RUSTSEC-2026-0385.
- Changed piped `--json` output to print one value per line (pimalaya-cli `55b6693`).
- Changed the message parser from mail-parser to [mime-rs](https://github.com/pimalaya/mime-rs), the one the himalaya CLI uses, for the message bodies the TUI renders.

  An HTML-only body is now rendered by html2text, undecorated: headings, lists and paragraphs keep their layout and long lines wrap at 80 columns, where they used to run together.

### Fixed

- Fixed the `Bcc` header being transmitted to every recipient over SMTP (RFC 5322 section 3.6.3). It is now removed before sending, as io-smtp 0.4 does by default.
- Fixed every Maildir mailbox failing to open with `path <root>/<root>/<mailbox> is not a directory`: the absolute path a mailbox is listed with is now reduced to its root-relative name before io-maildir resolves it, as the CLI does ([#22]).
- Fixed nested Maildir mailboxes sharing a last segment, such as `Archive` and `[Gmail]/Archive`, showing as identical rows: a Maildir is now named by its path relative to the root, as the CLI does ([#22]).
- Fixed message bodies and Maildir subjects in multi-byte charsets such as Shift_JIS, Big5, GBK or EUC-KR reading as replacement characters: they are now decoded. Reply and forward quotes still go through mml's parser and keep the old behaviour.

[#7]: https://github.com/pimalaya/himalaya-tui/issues/7
[#9]: https://github.com/pimalaya/himalaya-tui/issues/9
[#16]: https://github.com/pimalaya/himalaya-tui/issues/16
[#22]: https://github.com/pimalaya/himalaya-tui/issues/22
