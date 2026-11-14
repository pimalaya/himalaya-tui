---
cairn: change
id: the-tui-runs-on-himalaya
status: landed
created: 2026-11-14
---

# The TUI runs on himalaya

> Cross-repo change, same id in himalaya. Order: **himalaya** (a library target, `total` on `envelope list`, a session keepalive) → **himalaya-tui** (this side), pinning the revision landing it. Phase 1 of the plan of 2026-11-14: several-messages-at-once (#9) and then delete-moves-to-the-trash (#7) build on it, and onboarding-on-the-android-flow is phase 2.

## Why

The TUI drives mail through a backend layer of its own: src/shared, src/email, src/imap, src/jmap, src/maildir and src/smtp, about 2,900 lines, the copy it took of the retired io-email, plus the backend half of src/config.rs (the `imap`, `smtp`, `jmap` and `maildir` blocks and their `tls` and `sasl` tables). It duplicates himalaya's and drifts from it:

- The file is himalaya's, so every option himalaya adds to a backend block has to be mirrored here, or the TUI refuses the file at startup (configuration.md, "A configuration file valid for the CLI loads in the TUI").
- It speaks four backends where himalaya speaks eight (Gmail, Microsoft Graph, mbox and pimdir are missing), and none of the unified API's later work reaches it: mailbox roles and aliases, trash-first delete (#7), per-message errors (#9), the capability union.
- It pins pimalaya-cli 0.2, pimalaya-config 0.2.1, pimalaya-stream 0.3, io-imap 0.6 and io-jmap 0.3, where the family moved on to the 0.4 line and later.
- Its wizard tests connections with that same layer, and the family dropped its wizards on 2026-11-08 (himalaya the-wizard-leaves, 2026-11-11).

The owner decided on 2026-11-14 that the TUI becomes a frontend of himalaya, as himalaya-vim and himalaya-emacs are: it runs himalaya's own commands, in-process, as the REPL and the MCP server do, over a himalaya crate pinned by git revision under `[patch.crates-io]`. How himalaya is linked for releases is decided later. This phase covers an account the configuration resolves; with none, onboarding comes in phase 2.

## What

### Linking

- Cargo.toml: `himalaya = { version = "2.2", default-features = false, features = [...] }` under `[dependencies]` and `himalaya = { git = "https://github.com/pimalaya/himalaya", rev = "<rev>" }` under `[patch.crates-io]`, the revision being the one landing himalaya's side. The version requirement follows himalaya's own (2.2.1 on master on 2026-11-14), so its v3 bump moves both.
- Only the root manifest's `[patch.crates-io]` applies to a build: himalaya's own patch entries do not reach the TUI's. The TUI SHALL repeat them at the pinned revision (pimalaya-cli 0be1b1f and mime-rs 2591e7f on 2026-11-14), so the types crossing the boundary (the `Printer` trait, the error types) come from one crate. `cargo tree -d` shows no second pimalaya-cli.
- The backend features forward to himalaya's: `imap`, `smtp`, `jmap`, `gmail`, `msgraph`, `maildir`, `mbox` and `pimdir`, and so do `rustls-ring`, `rustls-aws`, `native-tls` and `vendored`. The default is himalaya's without `sieve`, which the TUI never drives.
- pimalaya-cli and pimalaya-config move to the 0.4 line himalaya uses, mime-rs to its 0.1 at himalaya's revision.

### The client module

One module holds everything that touches himalaya, src/client.rs by default. It must not be named `himalaya`, which would shadow the crate in every `use himalaya::...` of the TUI.

- **One session.** One `himalaya::session::Session` for the process, bound to the `-c` paths, the `-a` account and `--backend auto`. Its `SessionAccount` borrows the paths, so they are leaked once at startup and live for the process.
- **A command is an argv.** The TUI builds the argv, one element per value, parses it with a `no_binary_name` clap wrapper over `himalaya::cli::Command`, as the MCP server's `McpLine` does, and runs it with `Command::execute` over the session. Nothing is ever split as a shell splits a line, so a mailbox name with spaces or a whole message needs no quoting.
- **A printer of its own.** Output goes to the TUI's own `pimalaya_cli::printer::Printer`. It keeps each value and event as JSON, as `CapturePrinter` does, and the bytes `message read --raw` prints as they are, where `CapturePrinter` would base64 them.
- **The JSON contract, not the Rust types.** The TUI decodes the values into structs of its own mirroring the documented `--json` shapes (`himalaya json-schema`). It never names himalaya's output types: the coupling is the CLI's versioned contract, so moving himalaya out of process later changes only the transport.
- **Three outcomes.** `Ok` is a result. An `Err` carrying a `PartialOutcome` (`pimalaya_cli::error::partial_outcome_of`) is a result too, its `errors` list naming what failed. Any other `Err` is a failure, shown by its message, its code read with `code_of::<himalaya::error::ErrorCode>` where an action depends on it.
- **No command reads the terminal.** `pimalaya_cli::session::claim_stdin` runs at startup, so a command that would read its input from stdin is refused (`stdin-taken`).

### What each action runs

| Action | Command | Read from the answer |
| --- | --- | --- |
| List the mailboxes | `mailbox list` | `mailboxes[]`: `id`, `name`, `role` |
| List a page | `envelope list -m <id> -p <n> -s <rows>` | `envelopes`, `nextPage`, `total` |
| Read, reply, forward | `message read -m <id> --raw <msg>` | the bytes, decoded and templated as today |
| Copy, move | `message copy` or `message move -f <id> -t <id> <msg>` | `count`, `errors` |
| Add or remove a flag | `flag add` or `flag remove -m <id> -f <flag> <msg>` | `count`, `errors` |
| Send | `message send <file>` | `saved`, `queued` |
| Save a draft | `message add -m drafts -f draft <file>` | `queued` |
| Keep the connection | `Session::keep_alive` | nothing |

- A message to send or save goes through a private temporary file the command reads, never inline: himalaya reads the literal `\n` and `\r` sequences of an inline message as line endings, which would alter a body quoting code.
- A read leaves the flags alone (no `--seen`), as the `BODY.PEEK[]` of today does.
- A draft goes to the account's `drafts` alias or role, where today it goes to a mailbox literally named `Drafts`, which JMAP and Gmail spell otherwise.
- A sent message is copied where `message.send.save-copy` says, as the CLI copies it. Today the TUI keeps no copy.
- A queued write (pimdir) says it is queued rather than done, as the CLI does.
- A mailbox is passed by the `id` the listing gave. himalaya resolves `-m` through `mailbox.alias` first, so an alias key equal to another mailbox's id redirects it, as on the CLI (Open questions).

### Paging

A known `total` keeps `(page n/p)`. A `null` one, Gmail's, shows `(page n)`, and the next page is offered while `nextPage` is set. A next page that comes back empty, which a full last page leads to, leaves the page shown and marks it the last, so "Paging past the last page" holds on every backend. This replaces the fallback to the page length.

### Connection lifetime

- The session builds the unified client on the first command, keeps it, and drops it after a failed command, the next one building it again (himalaya repl.md, "A session connects once").
- The startup commands, `mailbox list` then the first page, run before the TUI takes the screen, behind the `Loading…` spinner, so a secret command prompting on the terminal prompts there, as it does today.
- A command that will build the client mid-session, the slot having been emptied by a failure, runs with the terminal handed back, as the system editor runs, and the screen is redrawn after it. A prompting secret command can then prompt again instead of drawing over the interface.
- Idle for `PING_INTERVAL` (60 s), the TUI calls `Session::keep_alive` where it called `ping`. A failure is logged, as a failed ping is today, and the next command reconnects.

### Configuration

- The account comes from himalaya (`himalaya::cli::resolve_account`): the one `-a` names, else the default, with the CLI's errors.
- The identity and the signature are read from himalaya's merged account (`email` and `display-name`, which himalaya's config also takes as `from` and `from-name`, then `signature` and `signature-delim`), so a value reads the same whichever binary composes.
- The TUI's own keys, `theme`, `keybinds`, `contact-command` (global and per account) and `contact-complete-key`, are read by a small struct over the same files, every other key ignored. No backend block is modelled here any more, so the parity rule leaves the spec.
- `-c` takes one path per flag, repeated to merge, as pimalaya-cli 0.4's shared flag and himalaya do, where the TUI split one value on `:`. `HIMALAYA_CONFIG` goes: the shared flag reads no environment variable, and himalaya never did. `-b/--backend` comes, as on the CLI.
- With no configuration, the TUI fails before taking the screen with himalaya's missing-configuration error, naming the path looked at and the sample.
- Removed until phase 2: the positional `EMAIL` seed, `--no-config`, the wizard and the fallback to it.
- Kept: `--from`, `--from-name`, `--keybinds`, the themes, the composer (edtui and mml), contact completion.

### What leaves

- Code: src/shared, src/email (the views read the client module's decoded structs instead), src/imap, src/jmap, src/maildir, src/smtp, src/wizard and the backend half of src/config.rs.
- Dependencies: io-imap and the `nom` feature pin under it, io-jmap, io-maildir, io-smtp, io-sasl, io-pim-discovery, pimalaya-stream, base64, url and uds_windows, then whatever only that code used (shellexpand, secrecy, rfc2047-decoder: checked one by one).

## Non-goals

- Several messages at once (#9) and delete (#7): their own changes, on top of this one.
- Onboarding without a configuration: phase 2, onboarding-on-the-android-flow.
- How himalaya is linked for releases (a published library, an out-of-process REPL): decided later, and the JSON coupling keeps either open.
- Running commands off the event loop: an action still blocks and announces itself first (interface.md).
- New-mail notifications: nothing in a session pushes.

## Depends on

himalaya, same change id: the library target, `total` on `envelope list`, `Session::keep_alive` and `Session::is_connected`, pinned by the revision landing them.

## Open questions

- **Alias collision.** A mailbox picked from the list is redirected when a `mailbox.alias` key equals its id. It needs an alias named after another existing mailbox, and the CLI behaves the same. A verbatim form of `-m` in himalaya would close it.
- **Binary size.** Every backend links in by default. Measure the release binary before and after, and trim the default features if it matters.
