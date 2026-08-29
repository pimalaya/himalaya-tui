---
cairn: log
change: credentials-unlock-once
landed: 2026-08-29
---

# An account's credentials resolve on one resolver, so a key unlocks once

pimalaya-config went 0.1 to 0.2, which moves `Secret::Command` off a built `std::process::Command` and onto a `CommandConfig`, the shape the configuration wrote: comparable, hashable, cheap to clone, and the key of the memo the release adds as `SecretResolver`. The two TOML shapes are unchanged, a string still deserializing to a shell line and an array to an argv, so no configuration file and no sample moved.

src/wizard/secret.rs builds those two variants directly, where it used to build a `Command` for a picker's argv and call `pimalaya_config::command::shell` for a hand-typed line. Its two validation tests are untouched and still reject an empty argv and a blank shell line.

A `&mut SecretResolver` now runs through the resolution path: `SaslConfig::try_into_sasl` and `jmap_http_auth` resolve on it rather than calling `Secret::get`, and `ImapClient::new`, `SmtpClient::new` and `JmapClient::new` take one. Each entry point that assembles an account builds one and drops it with the assembly, so nothing holds a plaintext credential beyond the connections it opened: `EmailClient::new` for a session, `check::test_account` for the account test, `imap_smtp::configure_discovered` for the two connection tests it runs itself.

That last one is where a user notices. The wizard seeds both keyring entries with the account name and offers to reuse the IMAP credential for SMTP, defaulting to yes; accepting used to spawn the one command twice, one `pass` or `gpg` prompt per test. It now spawns it once. The isolated call sites resolve on a resolver of their own, which is what they already did: the JMAP wizard's single test, and a blob download opening a session on a foreign authority long after the account was assembled.

The SMTP transport still connects on the first send, so its credential is still resolved then and not at startup, and a session that never sends spawns nothing for it. That is the one place where two blocks of a live account cannot share a spawn, and buying it with an eager unlock at startup would cost the user more than it saves.

`cairn/spec/configuration.md` gained "A credential command is spawned once per account resolution". The lazy SMTP transport requirement it sits next to is unchanged and still holds.
