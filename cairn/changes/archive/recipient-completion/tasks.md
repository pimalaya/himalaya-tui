---
cairn: tasks
change: recipient-completion
---

# Tasks

- [x] src/contact.rs: the command (substitution, quoting, spawn with timeout), the output parser, the address formatter, the header-field locator, the key parser.
- [x] src/config.rs: `contact-command` global and per account, `contact-complete-key` global.
- [x] src/cli.rs: resolve both into the model.
- [x] src/tui/model.rs: lookup in flight, open completion list, messages.
- [x] src/tui/update.rs: intercept the key, start the lookup, poll it, navigate and accept the list.
- [x] src/tui/app.rs: poll the lookup each iteration, shorter poll timeout while one runs.
- [x] src/tui/view.rs: the list under the cursor.
- [x] Tests: substitution and quoting, output parsing, formatting, field location.
- [x] config.sample.toml, CHANGELOG.md.
- [x] cargo fmt, cargo clippy, cargo test on all features.
- [x] Fold the delta into cairn/spec/contacts.md; write the log entry.
