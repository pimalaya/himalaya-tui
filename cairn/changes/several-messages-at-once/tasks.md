---
cairn: tasks
change: several-messages-at-once
---

# Tasks

- [x] the-tui-runs-on-himalaya landed.
- [x] src/tui/model.rs: the marked ids of the selected mailbox, in marking order; cleared on a mailbox change.
- [x] src/tui/update.rs: `space` toggles and moves on, crossing the page; `Esc` clears the marks first; the dialog's copy, move and flag actions take the marked ids or the focused one.
- [x] Outcomes: acted ids unmarked, failed ids kept marked, `Moved n of m messages` and the first failure; a move lists the page again with the cursor's row index clamped.
- [x] src/tui/view.rs: the marker column, the `n marked` title, the dialog title naming its targets; theme key `envelope-marked` in the theme config and the bundled themes.
- [x] Tests on a scratch Maildir: marks surviving paging, cleared on a mailbox change; one move of three marked envelopes across two pages; a partial move keeping the failed id marked.
- [x] README keys table; CHANGELOG under [Unreleased]: Added marks.
- [x] Fold the delta into envelopes.md, write the log entry, set `landed`.
- [ ] Answer #9.
