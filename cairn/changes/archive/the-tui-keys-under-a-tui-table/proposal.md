---
cairn: change
id: the-tui-keys-under-a-tui-table
status: landed
created: 2026-11-14
---

# The TUI keys under a tui table

> Cross-repo, after himalaya e4a65282 (unreleased), which refuses every unknown key at every level and skips exactly one foreign table, `tui`, at the top level and in each account. Owner decision of 2026-11-14.

## Why

The TUI's own keys (`keybinds`, `contact-command`, `contact-complete-key`, `theme` at the top level, `contact-command` in an account) sat beside himalaya's, each binary ignoring what the other owned. Himalaya now refuses a key it does not know, so a file shared with the TUI no longer loads in himalaya unless the TUI's keys move to the table himalaya leaves alone. The TUI also never caught its own typos: a misspelled `keybind` was silently dropped.

## What

- src/config.rs: `Config` reads a top-level `tui` table (`TuiConfig`: `keybinds`, `contact-command`, `contact-complete-key`, `theme`) and `AccountConfig` an account `tui` table (`AccountTuiConfig`: `contact-command`). Both refuse an unknown key; the rest of the file (himalaya's keys, the accounts and their `default`) stays read leniently.
- The old spots are no longer read. Himalaya refuses them before the TUI reads anything, since the account resolves through himalaya first.
- Docs: config.sample.toml, README (with a migration recipe), CHANGELOG (breaking), the src/main.rs header, CONTRIBUTING (the patch table).
- Cargo.toml: `himalaya`, `pimalaya-cli` and `mime-meta-language` patched to the local checkouts (../himalaya, ../cli, ../mml) carrying the new himalaya.

Not in scope: the TUI writes no configuration, so there is nothing to migrate automatically.
