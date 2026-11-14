---
cairn: change
id: delete-moves-to-the-trash
status: landed
created: 2026-11-14
---

# Delete moves to the trash

> Builds on the-tui-runs-on-himalaya and several-messages-at-once.

## Why

[#7](https://github.com/pimalaya/himalaya-tui/issues/7) asks for a delete action. Its two other points are settled: #5 was closed on 2026-05-31, and the copy and move dialogs filter mailboxes as one types since 1305f72 (2026-05-31). Delete was left out on 2026-05-31 because the shared API had none and the TUI could not tell which mailbox is the trash on every backend.

Both reasons are gone once the TUI runs on himalaya. `message delete` moves the messages to the trash, `mailbox.alias.trash` else the backend's trash role, and removes them for good when they are already there, flagging them `\Deleted` and expunging those UIDs alone (`UID EXPUNGE`), so no other message the mailbox flagged `\Deleted` goes with them. On an IMAP server without UIDPLUS it only flags them, an expunge still owing. The receipt says which happened (`action`: `moved-to-trash`, `deleted`, `flagged`).

## What

- `Delete` joins the envelope dialog, and the `Delete` key runs it from the envelope panel. It acts on the marked envelopes, else the focused one, in one `message delete -m <mailbox> <ids>`.
- In a mailbox whose `role` is `trash` (himalaya's listing applies `mailbox.alias.trash` to the roles), the TUI asks first, `Delete 3 messages for good?`, the one irreversible delete. Elsewhere it does not ask: moving to the trash is undone by moving back.
- The status bar reads the receipt: `Moved 3 messages to the trash`, `Deleted 3 messages for good`, or `Flagged 3 messages deleted, the server expunges them later`. A queued delete (pimdir) says it is queued. A partial one reads as several-messages-at-once says.
- The page is listed again, the cursor kept on its row index, clamped to the page.
- An account with no trash fails with himalaya's error, which names `mailbox.alias.trash`.

## Non-goals

- Emptying the trash, or expunging a mailbox.
- Undo, beyond moving back out of the trash.
- A delete that skips the trash anywhere but in the trash.
