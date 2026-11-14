---
cairn: log
change: delete-moves-to-the-trash
landed: 2026-11-14
---

# Delete moves to the trash

Delete, for [#7](https://github.com/pimalaya/himalaya-tui/issues/7), on top of the-tui-runs-on-himalaya and several-messages-at-once. Capability **envelopes** moved.

- **src/client.rs**: `delete_messages` runs `message delete -m <mailbox> -- <ids>`, decoded as `DeleteReceipt` (`action`: `moved-to-trash`, `deleted`, `flagged`, the receipt flattened beside it).
- **src/tui/model.rs**: `EnvelopeAction::Delete`, `Dialog::ConfirmDelete`, `Message::DeleteSelected`, `Blocking::Delete`, `selected_mailbox_role`.
- **src/tui/update.rs**: the `Delete` key and the dialog entry run `start_delete`: in a mailbox whose role is `trash` (himalaya's listing applying `mailbox.alias.trash`) it opens the confirmation, elsewhere it announces the delete. `do_delete` settles the marks, lists the page again, then says `Moved 1 message to the trash`, `Deleted 2 messages for good` or `Flagged 1 message deleted, the server expunges them later`. `receipt_status` now takes a phrase with the count in place.
- **src/tui/view.rs**: the confirmation, ` Delete n messages for good? `, offering `Delete for good` and `Cancel`.
- Docs: README, CHANGELOG.

Tests (src/tui/update.rs, a scratch Maildir with `mailbox.alias.trash`): a delete from the inbox moving without asking, a delete in the trash asking then removing for good, declining leaving everything, an account without a trash failing with himalaya's error naming `mailbox.alias.trash`.

Not done: a live check on the Fastmail test account, and #7 is not answered yet. The IMAP-without-UIDPLUS path is himalaya's, decoded here but not exercised against a server.
