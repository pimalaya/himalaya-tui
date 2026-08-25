---
cairn: log
change: identity-key-parity
landed: 2026-08-16
---

# The identity reads under either spelling

The himalaya CLI took back the two account fields it dropped in v2, `email` and `display-name`, so its composers can fill a `From` header ([issue 721](https://github.com/pimalaya/himalaya/issues/721)). The TUI had never lost them: it calls the same two things `from` and `from-name`.

One configuration file backs both binaries, so each now reads the other's names. Here that is two serde aliases on `AccountConfig`, `email` on `from` and `display-name` on `from_name`. An account written by the CLI's wizard opens in the TUI with its identity intact instead of composing anonymously.

The global `display-name` needed nothing: the TUI already spelled it that way, with `from-name` as its alias, and the CLI adopted exactly that pair.

Nothing else moved. The fallback wizard still builds its in-memory account with `from`/`from-name`, which never reaches disk, and the `--from` / `--from-name` flags keep their names.

## Capabilities moved

- **configuration**: the CLI-valid-file requirement now states that an account's identity reads under either spelling, with a scenario for a file the CLI wrote.
