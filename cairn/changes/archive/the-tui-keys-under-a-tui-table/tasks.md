---
cairn: tasks
change: the-tui-keys-under-a-tui-table
---

# Tasks

## Build

- [x] Cargo.toml: `himalaya`, `pimalaya-cli` and `mime-meta-language` patched by path to ../himalaya (e4a65282), ../cli (79c0d9e) and ../mml (ee0a380); no comment in the sections; Cargo.lock updated by building.
- [x] Build against the new himalaya: no compile break (the TUI reads only `--json`, whose shapes did not change, and uses neither the list frame nor the capability types).

## Configuration

- [x] Tests first, failing on the old struct: the `tui` tables read, an unknown key in them refused, the old spots not read.
- [x] `TuiConfig` and `AccountTuiConfig` with `deny_unknown_fields`; `Config` and `AccountConfig` stay lenient.
- [x] src/cli.rs reads the keybinds, the contact command and key, and the theme from the `tui` tables.
- [x] The sample test also loads config.sample.toml with every key line uncommented, in the TUI and in himalaya.

## Docs

- [x] config.sample.toml, README (migration recipe), CHANGELOG (**BREAKING**), src/main.rs header, CONTRIBUTING, the missing contact command status hint.
- [x] Spec folded into configuration.md, log written, change archived.

## Left

- [ ] The path patches need ../himalaya, ../cli and ../mml beside the repo: CI and the nix packages, which check out this repository alone, fail until they go back to git revisions or releases.
