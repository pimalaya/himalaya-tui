---
cairn: change
id: the-tui-reads-pimdir-draft-13
status: landed
created: 2026-11-15
---

# The TUI reads pimdir draft-13

> Line X17 of the CLI alignment of 2026-11-15 (.github cairn/changes/cli-alignment-2026-11-15). Follows himalaya 0154fd1f, which moved to io-pimdir 0.14.0 (pimdir storage-draft-13) and pimalaya-cli 0.6.0.

## Why

The TUI pinned himalaya 60e48de, which carries io-pimdir 0.13 (storage-draft-11) and pimalaya-cli 0.5. Stores are now rebuilt as storage-draft-13, which that io-pimdir refuses, so the TUI could no longer open a pimdir account. Between the two revisions himalaya also prints a missing `Subject:` as `null` in `envelope list`, where it printed `""`, which the TUI's envelope type refused, failing the whole page.

## What

- Cargo.toml: the `himalaya` patch moves to 0154fd1f0a19b7f7f93a8955842111ca70d1498a; pimalaya-cli to 0.6.0 (build and runtime), so the tree holds one copy; pimalaya-config to 0.5.2, himalaya's requirement. The `mime-meta-language` pin stays at 563dd51: himalaya does not depend on mml. Both `[patch]` entries stay until himalaya 3.0 and mml 2.0 are on crates.io (T2).
- Cargo.lock: himalaya's source, io-pimdir 0.13.0 to 0.14.0, pimalaya-cli 0.5.2 to 0.6.0, nothing else.
- src/email/envelope.rs: `subject` is `Option<String>`, `None` for himalaya's `null`; the envelope panel shows an empty cell for it.
- No other himalaya change reaches the TUI: the commands it runs (`mailbox list`, `envelope list`, `message read --raw`, `message copy|move|delete`, `flag add|remove`, `message send`, `message add`) keep their names and flags, `message add` dropping `saved` (the TUI never read it), and pimalaya-cli 0.6.0 leaves the `Printer` trait as it was.
