---
cairn: tasks
change: wizard-mirrors-the-cli
---

# Tasks

- [x] src/wizard/: search, discover, imap_smtp, jmap, local, mailbox, secret ported from the CLI; check holding the connection tests and the CAPABILITY probe; pacc, autoconfig and srv deleted.
- [x] src/wizard/configure.rs: the welcome, the account name, the save-or-append offer, and the account handed back whichever way it was answered.
- [x] src/config.rs: `render` with the CLI's `RENDER_ORDER`, `is_default` on the defaulted scalars, `MailboxConfig`, and the identity serialized under the CLI's spelling.
- [x] src/cli.rs: `footer!` on `--help`; the wizard call carrying the welcome path and the config paths; the account name now coming from the wizard.
- [x] Cargo.toml: io-pim-discovery `rfc8620`, shellexpand.
- [x] Tests: the rendered block parses back and orders its keys, an appended block keeps the existing one, a taken name gets a suffix, discovered aliases render.
- [x] README.md, config.sample.toml, CHANGELOG.md: the wizard writes now, and the two differences are named.
- [x] Fold the delta into [cairn/spec/configuration.md](../../spec/configuration.md); write [cairn/log/2026-08-25-wizard-mirrors-the-cli.md](../../log/2026-08-25-wizard-mirrors-the-cli.md).
