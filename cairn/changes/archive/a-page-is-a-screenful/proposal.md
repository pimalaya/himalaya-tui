---
cairn: change
id: a-page-is-a-screenful
status: landed
created: 2026-08-25
---

# A page is what the screen shows, and says so

Paging reaches the whole mailbox now, but two things about it read badly.

The page size is a constant fifty, and a terminal shows whatever it shows: thirty-five rows on a forty-row screen. Walking down the list therefore fills the screen, then scrolls into a second, half-empty one holding the fifteen rows that did not fit, and only then turns the page. The page is a unit the reader cannot see, so the scroll and the page fight each other.

The indicator reads `(1/4)`, which could be one message of four, one mailbox of four, or one of anything else.

## What changes

The page becomes exactly what the envelope panel can show: its height less its two borders and the column header. A page then fills the screen with nothing left over, and turning the page is the only way to move past it. `Model::envelope_capacity` carries what the last render measured, the view writes it, and the app adopts it after each draw, so a resized terminal re-pages the list around the envelope the cursor is on rather than snapping back to the top. Startup reads the terminal size directly for its first listing, since there is no frame to measure yet, and the first render corrects the guess if the layout ever disagrees.

The measurement is taken on the whole right panel rather than on the envelope pane alone: opening a message or the composer halves that pane, and re-paging the list under someone who just hit Enter would be worse than the scrolling this change removes.

The indicator becomes `(page 1/4)`.
