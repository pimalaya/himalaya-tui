---
cairn: change
id: credentials-unlock-once
status: landed
created: 2026-08-29
---

# Resolve an account's credentials on one resolver, so a key unlocks once

pimalaya-config 0.2 carries `SecretResolver`, a `Secret::get` that remembers what it spawned. It exists because a configuration usually points several fields at one credential, and resolving them field by field reads that credential as many times as it appears. For a `pass` or `gpg` entry each read is a key unlock, which the user answers by hand.

This binary walks straight into that. The wizard seeds the IMAP and the SMTP keyring entry with the same account name, then offers "Use the same credentials for SMTP?" and defaults it to yes; accepting hands the very same `SaslConfig` to both blocks. The two connection tests that follow then spawn that one command twice, one prompt each, in a flow the user has already answered. `check::test_account` does the same for an account the local flows built, testing IMAP or JMAP and then SMTP.

## What changes

`Secret::Command` now carries a `CommandConfig` rather than a built `std::process::Command`, so src/wizard/secret.rs builds `CommandConfig::Argv` for a picker's argv and `CommandConfig::Shell` for a hand-typed line, where it used to build a command and call `shell`. The two TOML shapes are unchanged, so no configuration file and no sample moves.

A `&mut SecretResolver` is threaded through the resolution path: `SaslConfig::try_into_sasl` and `jmap_http_auth` take one and resolve on it, and so do the three client constructors, `ImapClient::new`, `SmtpClient::new` and `JmapClient::new`. Each entry point assembling an account builds one and drops it with the assembly: `EmailClient::new` for a session, `check::test_account` for the account test, `imap_smtp::configure_discovered` for the pair of tests it runs itself. The isolated call sites, the JMAP wizard's single test and a blob download opening a session on a foreign authority, resolve on a resolver of their own, which is what they already did.

The SMTP transport keeps connecting on the first send, so its credential is still resolved then and not at startup: a session that never sends spawns nothing for it. That is the one place where two blocks of a live account cannot share a spawn, and paying an eager unlock to close it would cost more than it saves.

Nothing else moves. Same prompts, same TOML, same errors.
