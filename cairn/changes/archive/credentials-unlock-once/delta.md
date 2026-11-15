---
cairn: delta
change: credentials-unlock-once
---

## ADDED Requirements

### Requirement: A credential command is spawned once per account resolution

A configuration is free to point several fields at one credential, and the wizard actively produces such a file: it seeds the IMAP and the SMTP keyring entry with the same account name and offers to reuse the IMAP credential for SMTP. Every secret resolved while one account is assembled SHALL go through a single resolver, which spawns each distinct command once and hands its value to every field naming it. Distinct is the command as the configuration wrote it, so a shell line and the argv spelling that runs it are two commands.

The resolver holds plaintext, so it SHALL live no longer than the assembly that built it. It is never process-wide and no client keeps one.

#### Scenario: IMAP and SMTP name one credential command

Given an account whose `imap` and `smtp` blocks name the same password command, when the wizard tests both connections, then the command is spawned once and the second test reuses its value.

#### Scenario: A credential is a literal

Given a `raw` secret, when it is resolved, then the value passes through with nothing spawned and nothing memoized.
