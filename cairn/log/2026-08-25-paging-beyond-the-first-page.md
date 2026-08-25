---
cairn: log
change: paging-beyond-the-first-page
landed: 2026-08-25
---

# The envelope list pages through the whole mailbox

[Issue #12](https://github.com/pimalaya/himalaya-tui/issues/12) reported an inbox stuck at fifty messages. Paging was dead on every backend and had been since the interface was written, Gmail and IMAP being where it happened to be noticed.

`load_envelopes` had no total, so it took the length of the page it had just received for one. A full page therefore reported fifty messages in a mailbox of thousands, `Model::total_pages` divided fifty by fifty and answered one, and both readers of that number closed: `next_envelope_page` refused to advance past the only page it believed existed, and the view hid the `(page/total)` indicator because there was nothing to count. `next_item` clamped at the end of the loaded page, so `j` and the down arrow hit the same wall. A mailbox under the page size looked right only because the guess happened to be true there.

The total now comes from the backend, which held it all along. `list_envelopes` returns an `EnvelopeList` carrying the page and the count: IMAP hands back the `EXISTS` its SELECT already answered and already sizes the window with, JMAP the `total` it already asks for through `calculateTotal`, Maildir the entry count taken before the slice. `EmailClient` passes the pair through and `load_envelopes` assigns it. RFC 8621 §5.5 leaves `total` optional even when calculated, so a JMAP server withholding it falls back to the page length, which is the old behaviour for that server alone. None of the three windowing helpers moved: `compute_window`, `compute_position_limit` and `paginate` were already correct for page two and beyond, and now have tests saying so.

Navigation crosses the boundary. `Message::LoadEnvelopes` carries an `EnvelopeLanding`, so `Next` on the last row loads the following page and lands on its first, `Previous` on the first row loads the preceding page and lands on its last, and nothing new is stored on the model to remember where to land. The paging keys keep landing on the first row. The view needed no change: it already clamps the scroll offset to the selected index on every render, so a selection landing at the bottom of a page scrolls into view on its own.

The envelopes capability is new: cairn/spec/envelopes.md states that a listing reaches every message, that the count comes from the backend, and how a cursor behaves at either end of a page. The configuration capability did not move.
