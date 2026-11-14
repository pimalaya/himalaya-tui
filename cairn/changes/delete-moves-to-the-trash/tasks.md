---
cairn: tasks
change: delete-moves-to-the-trash
---

# Tasks

- [x] the-tui-runs-on-himalaya and several-messages-at-once landed.
- [x] src/tui/model.rs: `EnvelopeAction::Delete`; a confirmation dialog.
- [x] src/tui/update.rs: the `Delete` key in the envelope panel; `message delete -m <mailbox> <ids>` on the marked or focused envelopes; the confirmation when the selected mailbox's role is `trash`.
- [x] Decode the receipt's `action`, `count`, `queued` and `errors`; the three status lines; the page listed again with the cursor's row index clamped.
- [x] Tests on a scratch Maildir with a `mailbox.alias.trash`: a delete moving to the trash; a delete in the trash asking first, then removing for good; declining leaves everything in place; no trash configured fails with himalaya's error.
- [ ] A live check on the Fastmail test account (JMAP trash role) in a scratch mailbox.
- [x] README keys table; CHANGELOG under [Unreleased]: Added delete.
- [x] Fold the delta into envelopes.md, write the log entry, set `landed`.
- [ ] Answer #7 and close it.
