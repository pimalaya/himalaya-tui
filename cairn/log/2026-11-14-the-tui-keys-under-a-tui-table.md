---
cairn: log
change: the-tui-keys-under-a-tui-table
landed: 2026-11-14
---

# The TUI keys under a tui table

The TUI's own keys moved under `tui` tables, after himalaya e4a65282 started refusing every key it does not know but those tables. Owner decision of 2026-11-14. Capabilities **configuration** and **contacts** moved.

- **src/config.rs**: `Config.tui` (`TuiConfig`: `keybinds`, `contact-command`, `contact-complete-key`, `theme`) and `AccountConfig.tui` (`AccountTuiConfig`: `contact-command`), both `deny_unknown_fields`; `Config` and `AccountConfig` stay lenient. The old spots are no longer read.
- **src/cli.rs**: the model reads the keybinds, the contact command and key, and the theme from the `tui` tables, the account's command still winning.
- **Cargo.toml**: `himalaya`, `pimalaya-cli` and `mime-meta-language` patched by path to ../himalaya (e4a65282), ../cli (79c0d9e) and ../mml (ee0a380). No compile break: the TUI reads only `--json`, unchanged, and uses neither the shared list frame nor the capability types.
- Docs: config.sample.toml, README (migration recipe), CHANGELOG (**BREAKING**), CONTRIBUTING, the src/main.rs header, the missing contact command status hint.

Tests (src/config.rs, files in a scratch directory), the first three failing on the old struct: the `tui` tables read with the same values (and himalaya's `resolve_account` loads the file), an unknown key in `[tui]`, `[tui.theme]` or `[accounts.work.tui]` refused naming it, the old top-level and account keys not read and refused by himalaya. The sample test also loads config.sample.toml with every key line uncommented.

Left: CI and the nix packages check out this repository alone, so they fail on the path patches until those go back to git revisions or releases.
