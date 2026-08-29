---
cairn: tasks
change: credentials-unlock-once
---

# Tasks

- [x] Cargo.toml: pimalaya-config `0.1` to `0.2`, same features.
- [x] src/wizard/secret.rs: `command_secret` and `shell_secret` build a `CommandConfig`, and `std::process::Command` with `pimalaya_config::command::shell` are gone.
- [x] src/config.rs: `SaslConfig::try_into_sasl` takes a `&mut SecretResolver` and resolves every credential on it.
- [x] src/jmap/client.rs: `jmap_http_auth` and `JmapClient::new` take one; a blob download on a foreign authority resolves on its own.
- [x] src/imap/client.rs, src/smtp/client.rs: `new` takes one.
- [x] src/shared/client.rs: `EmailClient::new` builds the resolver and hands it to `select_storage`; the SMTP transport still resolves at its first send.
- [x] src/wizard/check.rs, src/wizard/imap_smtp.rs, src/wizard/jmap.rs: one resolver per account test, one for the IMAP+SMTP pair, one of its own for the JMAP test.
- [x] Tests: one command named by two blocks is spawned once.
- [x] CHANGELOG.md: the authentication bullet names the single unlock.
- [x] cargo fmt, cargo check and cargo test on all features, plus the per-backend feature builds.
- [x] Fold the delta into [cairn/spec/configuration.md](../../spec/configuration.md); write [cairn/log/2026-08-29-credentials-unlock-once.md](../../log/2026-08-29-credentials-unlock-once.md).
