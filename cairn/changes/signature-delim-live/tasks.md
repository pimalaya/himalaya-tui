---
cairn: tasks
change: signature-delim-live
---

# Tasks

- [x] src/cli.rs: `signature_block` assembling the separator and the signature, `DEFAULT_SIGNATURE_DELIM`, and the global and account `signature-delim` resolved alongside `signature`.
- [x] src/tui/model.rs: the model field documented as the assembled block, empty when the account declares no signature.
- [x] src/config.rs: both fields documented, `signature` as the text alone and `signature-delim` as the verbatim separator.
- [x] Tests: the default separator, a custom one written verbatim, and an unset or blank signature yielding no block at all.
- [x] config.sample.toml: the two keys documented under the new meaning, and the shared-file note, the identity and the signature no longer being TUI-only.
- [x] CHANGELOG.md: the initial-release bullet for the two keys.
- [x] Fold the delta into [cairn/spec/configuration.md](../../spec/configuration.md); write [cairn/log/2026-08-16-signature-delim-live.md](../../log/2026-08-16-signature-delim-live.md).
