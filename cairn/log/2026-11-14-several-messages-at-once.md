---
cairn: log
change: several-messages-at-once
landed: 2026-11-14
---

# Several messages at once

Marks, for [#9](https://github.com/pimalaya/himalaya-tui/issues/9), on top of the-tui-runs-on-himalaya. Capability **envelopes** moved.

- **src/tui/model.rs**: `marked`, the ids in marking order, belonging to the selected mailbox; `action_ids` (the marked ids, else the focused one), `is_marked`; `Message::ToggleMark`.
- **src/tui/update.rs**: `Space` in the envelope panel toggles and moves down through `next_item`, so it crosses pages. `Esc` clears the marks before the rest of its cascade, and selecting or leaving a mailbox clears them. Copy, move and flag take `action_ids` in one himalaya command; `settle_marks` keeps marked the ids the receipt's `errors` name and unmarks the rest. A flag change lands on every acted envelope of the page. `receipt_status` phrases `Moved 1 message`, or `Moved 2 of 3 messages, error: <first>`.
- **A move reloads inline**: the reload after a move used to go through `LoadEnvelopes`, whose announce-then-run cycle clears the status bar, so the move's own status was lost (`marks_cross_pages_and_one_move_carries_them_all` caught it). `do_move` now calls `load_envelopes` directly, then sets its status, and `load_envelopes` steps back over an empty page in a loop rather than through a follow-up message.
- **src/tui/view.rs**: a `+` in the flags column and `[theme.envelope-marked]` patched over the row's style (each preset carries one); `(n marked)` in the envelope panel's and the Actions dialog's titles.
- Docs: README keybindings, CHANGELOG, config.sample.toml.

Tests (src/tui/update.rs, a Model over a scratch Maildir driven through `apply_all`): marks crossing pages and one move carrying three ids, a partial move keeping the failed id marked with `Moved 1 of 2 messages, error: `, a flag reaching every marked envelope, `Esc` clearing the marks, another mailbox clearing them.

Not done: the key is not configurable (`space`), and #9 is not answered yet.
