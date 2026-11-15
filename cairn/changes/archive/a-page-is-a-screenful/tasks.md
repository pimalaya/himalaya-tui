---
cairn: tasks
change: a-page-is-a-screenful
---

# Tasks

- [x] src/tui/view.rs: `envelope_capacity` names the row arithmetic, the right panel records it, and the title reads `(page n/p)`.
- [x] src/tui/model.rs: `envelope_capacity` beside the page size; `EnvelopeLanding::Index` for a re-paged cursor.
- [x] src/tui/update.rs: `adopt_envelope_capacity` takes the measurement as the page size and re-pages around the selection.
- [x] src/tui/app.rs: adopt after each draw.
- [x] src/cli.rs: first page sized from the terminal, before there is a frame to measure.
- [x] README and CHANGELOG stop saying fifty.
- [x] cargo fmt, clippy, test; a live run against a 120-message Maildir on a 40-row terminal.
- [x] Fold cairn/spec/envelopes.md and write cairn/log/2026-08-25-a-page-is-a-screenful.md.
