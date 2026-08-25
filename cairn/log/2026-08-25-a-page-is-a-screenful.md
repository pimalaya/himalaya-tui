---
cairn: log
change: a-page-is-a-screenful
landed: 2026-08-25
---

# A page is a screenful, and the title says so

The page size was a constant fifty against a panel showing whatever the terminal gave it, thirty-five rows on a forty-row screen. Walking down the list filled the screen, scrolled into a second half-empty one holding the fifteen rows that did not fit, and only then turned the page: the page was a unit the reader could not see. It is now exactly what the panel can show, its height less its two borders and the column header, so a page fills the screen with nothing left over and turning it is the only way past it.

`Model::envelope_capacity` carries what the last render measured, `view::envelope_capacity` names the arithmetic both the scroll offset and the page size read, and `update::adopt_envelope_capacity` takes the measurement as the page size after each draw. A resized terminal therefore re-cuts the page around the envelope the cursor is on rather than snapping back to the top, which is what `EnvelopeLanding::Index` was added for: the selection's absolute position divided by the new size gives the page, the remainder the row.

The measurement is taken on the whole right panel rather than on the envelope pane alone. Opening a message or the composer takes the pane's lower half, and re-paging the list under someone who just pressed Enter would be worse than the scrolling this removes, so what the page holds follows the terminal and nothing else on screen.

Startup has no frame to measure, so it reads the terminal size and derives the same arithmetic for its first listing. A terminal that cannot be measured, or one too small to hold a row, still asks for one envelope; the first render corrects the estimate if it ever missed, at the cost of a second listing. On a forty-row terminal the guess and the measurement agree, so the session lists once.

The indicator reads `(page 1/4)` where it read `(1/4)`, which could have been one message of four.

Verified live against a 120-message Maildir on a forty-row terminal: thirty-five rows drawn, `Envelopes - INBOX (page 1/4)` in the title, one listing at startup, `Ctrl-v` and `Ctrl-u` stepping the page, `j` on the last row loading page two at its first row and `k` there loading page one at its last.

The envelopes capability moved: a page is a screenful, it follows a resize and not a panel opening, and the title names it.
