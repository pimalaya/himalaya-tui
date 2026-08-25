---
cairn: tasks
change: identity-key-parity
---

# Tasks

- [x] src/config.rs: `email` and `display-name` as serde aliases of `AccountConfig::from` and `AccountConfig::from_name`.
- [x] config.sample.toml: both spellings documented on the two keys, and the shared-file note updated, the identity no longer being TUI-only.
- [x] CHANGELOG.md: the shared-configuration bullet names the identity keys.
- [x] Fold the delta into [cairn/spec/configuration.md](../../spec/configuration.md); write [cairn/log/2026-08-16-identity-key-parity.md](../../log/2026-08-16-identity-key-parity.md).
