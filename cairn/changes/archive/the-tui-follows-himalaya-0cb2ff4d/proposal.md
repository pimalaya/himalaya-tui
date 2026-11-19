---
cairn: change
id: the-tui-follows-himalaya-0cb2ff4d
status: landed
created: 2026-11-19
---

# The TUI follows himalaya 0cb2ff4d

> Prepares the first release, which waits on himalaya 3.0 and mml 2.0 (line T2 of the CLI alignment of 2026-11-15, .github cairn/changes/cli-alignment-2026-11-15). Follows himalaya 0cb2ff4d (pushed 2026-11-18) and mml 1b9f3f2.

## Why

The TUI pinned himalaya d98528b5 (io-pimdir 0.14.1, pimdir storage-draft-13) and mml 9a214d8. himalaya now reads storage-draft-14 (io-pimdir 0.17.0) and refuses a draft-13 store, so a pimdir account no longer opened beside a current neverest. himalaya also moved to pimalaya-cli 0.6.1 and pimalaya-config 0.6.0, and a send whose copy fails after the message went out is now a result (`errors` in the receipt, exit 3) where it was a failure: the TUI read it as a plain `Message sent`, the failed copy dropped.

## What

- Cargo.toml: the `himalaya` patch moves to 0cb2ff4d41da4f0d94786b0facab592365f14932, the `mime-meta-language` patch to 1b9f3f21e4b9be3131f0a00f1159f9baa881b235; pimalaya-cli to 0.6.1 (build and runtime), pimalaya-config to 0.6.0, so the tree holds one copy of each. Both `[patch]` entries stay until himalaya 3.0 and mml 2.0 are on crates.io (T2). The `himalaya` requirement stays `2.2`, the pinned revision still being 2.2.1.
- Cargo.lock: himalaya and mml sources, io-pimdir 0.17.0, io-jmap 0.10.1, io-msgraph 0.9.3, ical-rs 0.7.1, jiff 1 (and its parts), rusqlite 0.43 with libsqlite3-sys 0.40, pimalaya-cli 0.6.1, pimalaya-config 0.6.0; defmt leaves the tree.
- src/client.rs: `SendReceipt` reads `errors`. src/tui/update.rs: a send whose receipt names an error says `Message sent, error: <first error>` and closes the composer, sending again sending twice.
- No other himalaya change reaches the TUI. The commands it runs (`mailbox list`, `envelope list -p -s`, `message read --raw`, `message copy|move|delete`, `flag add|remove`, `message send`, `message add -f draft`) keep their names, flags and answers: cursor paging is `pimdir search`'s alone, `envelope list` keeps page numbers; the relative and instant search dates, the protocol refusals printed as results and the account-level `tls` table concern commands and keys the TUI does not run or model (its `[accounts.<name>]` reading tolerates himalaya's keys). The `Printer` trait of pimalaya-cli 0.6.1 is unchanged.
