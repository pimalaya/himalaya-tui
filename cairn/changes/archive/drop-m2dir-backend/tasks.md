---
cairn: tasks
change: drop-m2dir-backend
---

# Tasks

- [x] Cargo.toml: drop the `m2dir` feature, the io-m2dir dependency and `m2dir` from `default`.
- [x] src/m2dir/ removed; src/main.rs drops the module and the two doc mentions.
- [x] src/shared/client.rs: the `M2dir` variant, its arms and its constructor branch go.
- [x] src/config.rs: `AccountConfig::m2dir`, `M2dirConfig` and the `m2dir` entry in `RENDER_ORDER` go.
- [x] src/wizard/: local.rs removed (nothing left to detect or pick), its `MaildirConfig` built in discover.rs, and the m2dir check arm gone.
- [x] Docs: README, CONTRIBUTING, config.sample.toml and the CHANGELOG backend bullet stop naming m2dir.
- [x] cargo fmt, clippy and test, plus the reduced feature-set builds.
- [x] Fold cairn/spec/configuration.md and write cairn/log/2026-08-25-drop-m2dir-backend.md.
