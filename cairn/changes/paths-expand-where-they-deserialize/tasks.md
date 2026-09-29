---
cairn: tasks
change: paths-expand-where-they-deserialize
---

# Tasks

- [x] src/config.rs: `MaildirConfig::root` deserializes through `shell_expanded_path`.
- [x] src/config.rs: `TlsConfig::cert` deserializes through a private `opt_shell_expanded_path`, carrying a TODO naming the shared helper it waits on.
- [x] src/config.rs: `downloads_dir` removed from `Config` and `AccountConfig`.
- [x] Tests: a tilde path reaches the field expanded, an omitted one stays unset, and a CLI-only `downloads-dir` still loads.
- [x] config.sample.toml: the two `downloads-dir` entries removed, `maildir.root` and `tls.cert` documented as shell-expanded.
- [x] CHANGELOG.md: the backends bullet names the expansion.
- [x] cargo fmt, cargo check, cargo test and cargo clippy on all features, plus the per-backend feature builds.
- [x] The live configuration at ~/.himalayarc loads through the binary.
- [x] Fold the delta into [cairn/spec/configuration.md](../../spec/configuration.md); write [cairn/log/2026-08-29-paths-expand-where-they-deserialize.md](../../log/2026-08-29-paths-expand-where-they-deserialize.md).
