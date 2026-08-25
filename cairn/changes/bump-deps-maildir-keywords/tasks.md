---
cairn: tasks
change: bump-deps-maildir-keywords
---

# Tasks

- [x] Cargo.toml: io-imap `0.6`, io-maildir `0.3`; `cargo update` for the rest.
- [x] src/maildir/backend.rs: `read_entries` takes the Maildir; `envelope_from_entry` reads `MaildirFullEntry::flags` through the new `flag_from_maildir`, and `parse_filename_flags` with `flag_from_char` are gone.
- [x] Tests: the two flag mappings round-trip, keyword included, and a resolved keyword keeps its wire spelling.
- [x] CHANGELOG.md: the backend bullet names the Maildir keyword conventions.
- [x] cargo clippy, cargo test, cargo deny and the per-backend feature builds.
- [x] No capability file moves; write [cairn/log/2026-08-25-bump-deps-maildir-keywords.md](../../log/2026-08-25-bump-deps-maildir-keywords.md).
