---
cairn: change
id: paging-beyond-the-first-page
status: landed
created: 2026-08-25
---

# Page through the whole mailbox, not just the first fifty

[Issue #12](https://github.com/pimalaya/himalaya-tui/issues/12) reports an inbox showing fifty messages and nothing below them. It is not a Gmail or an IMAP problem: paging is dead on every backend, and has been since the interface was written.

`load_envelopes` has no total to work from, so it approximates one with the length of the page it just received. A full page therefore reports fifty messages in a mailbox of thousands, `Model::total_pages` divides fifty by fifty and answers one, and the two guards reading it both close: `next_envelope_page` refuses to advance because the page it would move to is past the only page it believes exists, and the view hides the `(page/total)` indicator because there is nothing to count. The keys work, the state machine answers no, and nothing on screen says why. A mailbox under the page size looks correct only because the approximation happens to be true there.

`j` and the down arrow do not rescue it either: `next_item` clamps at the end of the loaded page, so the wall is the same whichever key reaches it.

## What changes

Every backend already holds the number the model needs and drops it on the floor. IMAP reads `exists` from the SELECT it just issued and uses it to size the window; JMAP asks for `calculateTotal` and gets `total` back in the same response; Maildir counts the entries before paginating them. So this is plumbing, not protocol: `list_envelopes` returns an `EnvelopeList` carrying the page and that total, `EmailClient` passes it through, and `load_envelopes` assigns it instead of guessing. The window arithmetic in all three backends is already correct for page two and beyond, and none of it moves.

A JMAP server is free to omit `total` even when asked, so that case falls back to the page length, which is the old behaviour for that server alone rather than for everyone.

Navigation gains the roll-over the report implies. At the last row of a page, `Next` loads the following page and lands on its first row; at the first row, `Previous` loads the preceding page and lands on its last. `Message::LoadEnvelopes` carries which end to land on, so nothing new is stored on the model. The paging keys keep landing on the first row, as they do now.
