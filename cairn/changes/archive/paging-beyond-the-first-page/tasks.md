---
cairn: tasks
change: paging-beyond-the-first-page
---

# Tasks

- [x] src/email/envelope.rs: add `EnvelopeList`, the page and the total the backend reported.
- [x] src/imap/backend.rs: return `exists` beside the window it already sizes with it.
- [x] src/jmap/backend.rs: return `output.total`, falling back to the page length when the server omits it.
- [x] src/maildir/backend.rs: return the entry count taken before pagination.
- [x] src/shared/client.rs: `list_envelopes` hands back the `EnvelopeList`.
- [x] src/tui/update.rs: `load_envelopes` assigns the reported total; `Next` and `Previous` roll over the page boundary.
- [x] src/tui/model.rs: `LoadEnvelopes` carries the end to land on.
- [x] Tests: the IMAP window past page one and at the exactly-full boundary, and the Maildir slice past page one.
- [x] CHANGELOG.md; cargo fmt, clippy, test and the reduced feature-set builds.
- [x] Fold cairn/spec/envelopes.md and write cairn/log/2026-08-25-paging-beyond-the-first-page.md.
