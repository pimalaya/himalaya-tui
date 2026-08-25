---
cairn: change
id: identity-key-parity
status: landed
created: 2026-08-16
---

# Read the identity under the CLI's spelling too

The TUI and the himalaya CLI read one configuration file, and until now only the TUI carried the account's identity: `from` and `from-name`, the address it sends as and the name that address goes by. The CLI dropped both in v2 and composed without a `From` header, which [issue 721](https://github.com/pimalaya/himalaya/issues/721) reported.

The CLI takes them back under the names v1 used, `email` and `display-name`, and reads `from`/`from-name` as aliases so a file the TUI's user wrote already works. This is the other half: the TUI reads `email` and `display-name` as aliases of its own two keys, so a file written under the CLI's spelling opens here with its identity intact instead of composing anonymously.

## What changes

Two serde aliases on `AccountConfig`, and nothing else. The global `display-name` needed none: the TUI already spelled it that way, with `from-name` as its alias.

The TUI keeps writing `from`/`from-name` in the fallback wizard's in-memory account, which never reaches disk anyway, and its `--from` / `--from-name` flags keep their names.
