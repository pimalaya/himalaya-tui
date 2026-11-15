---
cairn: change
id: several-messages-at-once
status: landed
created: 2026-11-14
---

# Several messages at once

> Builds on the-tui-runs-on-himalaya; delete-moves-to-the-trash builds on this one.

## Why

[#9](https://github.com/pimalaya/himalaya-tui/issues/9) asks to select several messages and move them together. Every envelope action acts on the focused envelope alone: src/tui/update.rs passes `&[&envelope.id]` to the copy, move and flag calls (:971, :994, :1021). himalaya's `message copy`, `message move`, `flag add`, `flag remove` and `message delete` take several ids, count each once and report each one that failed (`errors`, coded `message-not-found` and the others), so the work is the TUI's alone.

## What

### Marks

- `space` in the envelope panel toggles the mark of the focused envelope and moves the cursor to the next one, crossing the page boundary as `j` does.
- A marked row carries a marker in a column of its own and the theme style `envelope-marked`. The panel title says how many are marked (`3 marked`) beside the page.
- `Esc` in the envelope panel clears the marks first when any are set, before the rest of its cascade.

### Scope

- Marks belong to the selected mailbox, as the ids they hold do. They survive paging and a resize, and clear when another mailbox is selected.

### Acting

- The envelope dialog acts on the marked envelopes when any are set, else on the focused one, for copy, move and adding or removing a flag; its title says which (`3 marked`, or the focused subject). Read, reply, reply all and forward always act on the focused envelope.
- One command carries every marked id, in the order they were marked.
- After the command, the ids it acted on are unmarked. Those its `errors` name stay marked, and the status bar says what happened, `Moved 4 of 5 messages`, then the first failure.
- A flag change shows on every acted envelope the page holds; envelopes on other pages show theirs when their page loads.
- A move lists the page again, since it lost rows on this page or others, and keeps the cursor on its row index, clamped to the page.

## Non-goals

- Marking a range, every envelope of a page, or by a search.
- Marks across mailboxes: a command acts in one mailbox.
- Undo.

## Open questions

- The key. `space` is free in the envelope panel and reads as a toggle; mutt's `t` is the other habit. A configurable key belongs with a keybindings change of its own.
