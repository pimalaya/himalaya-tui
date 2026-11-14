---
cairn: spec
capability: envelopes
---

# Envelopes

How the interface lists a mailbox's messages and moves through them.

### Requirement: The envelope list reaches every message in the mailbox

A listing SHALL be paged. The page count SHALL come from the `total` himalaya's `envelope list` reports for the mailbox. When it reports none, the panel SHALL name the page alone and offer the next one while `nextPage` is set, and a next page coming back empty SHALL leave the page shown and mark it the last.

A page SHALL hold exactly the envelopes the panel can show at full height, so that a page is never partly off screen, and SHALL follow the terminal when it is resized, keeping the cursor on the envelope it was on. The panel SHALL name the page it is on, as `(page n/p)`, or `(page n)` without a total, whenever the mailbox holds more than one.

#### Scenario: A mailbox larger than one page

Given a mailbox holding more messages than the panel can show, when its envelopes are listed, then the page count is the total divided by the rows the panel holds, rounded up, and the panel title reads `(page 1/p)`.

#### Scenario: A page that is exactly full

Given a mailbox holding exactly one page of messages, when its envelopes are listed, then the page count is one and no further page is offered.

#### Scenario: A backend reporting no total

Given a Gmail mailbox holding exactly two pages, when the third page is asked for from the second, then it comes back empty, the second page stays on screen, and no further page is offered.

#### Scenario: Paging past the last page

Given the last page of a mailbox, when the next page is asked for, then nothing moves and the page already shown stays.

#### Scenario: The terminal is resized

Given a listing on screen, when the terminal is resized, then the page is re-cut to the new height and the envelope the cursor was on stays selected.

#### Scenario: A message is opened below the list

Given a listing on screen, when a message or the composer opens under it, then the page is not re-cut and the selection does not move.

### Requirement: Navigation crosses the page boundary

Moving past the end of a page SHALL load the next one and select its first envelope, and moving before the start of a page SHALL load the previous one and select its last. The paging keys SHALL land on the first envelope of the page they reach, whichever direction they moved in.

#### Scenario: Moving down from the last envelope of a page

Given the last envelope of a page that is not the last, when the next envelope is asked for, then the following page is loaded and its first envelope is selected.

#### Scenario: Moving up from the first envelope of a page

Given the first envelope of a page that is not the first, when the previous envelope is asked for, then the preceding page is loaded and its last envelope is selected.

#### Scenario: Moving down from the last envelope of the last page

Given the last envelope of the last page, when the next envelope is asked for, then the selection does not move.

### Requirement: Envelopes can be marked

`space` in the envelope panel SHALL toggle the mark of the focused envelope and move to the next one, crossing the page boundary as moving down does. Marked rows SHALL show a marker and the panel title how many are marked. The marks SHALL belong to the selected mailbox: they survive paging and a resize, and SHALL clear when another mailbox is selected or `Esc` is pressed in the envelope panel.

#### Scenario: Marking across pages

Given the last envelope of a page marked, when `space` is pressed on it, then the next page loads with its first envelope focused, and coming back to the first page shows the mark still set.

#### Scenario: Another mailbox

Given three marked envelopes, when another mailbox is selected, then no envelope is marked.

### Requirement: An action reaches every marked envelope

Copy, move and adding or removing a flag SHALL act on the marked envelopes when any are set, else on the focused one, in one command carrying every id. Read, reply, reply all and forward SHALL act on the focused envelope alone.

#### Scenario: Moving three marked envelopes

Given three marked envelopes, two on the first page and one on the second, when they are moved to another mailbox, then one `message move` carries the three ids, the page is listed again without them, and no envelope stays marked.

### Requirement: A partial action keeps what failed marked

After an action on marked envelopes, the ones it acted on SHALL be unmarked and the ones its `errors` name SHALL stay marked, the status bar saying how many of how many were acted on and the first failure.

#### Scenario: One of three is gone

Given three marked envelopes, one of which another client has since moved, when they are moved, then two move, the third stays marked, and the status bar reads `Moved 2 of 3 messages` followed by its failure.

### Requirement: Delete moves to the trash first

Delete, from the envelope dialog or the `Delete` key, SHALL run himalaya's `message delete` on the marked envelopes, else the focused one: the messages move to the account's trash, and only messages already in the trash are removed for good. In a mailbox whose role is `trash`, the TUI SHALL ask before deleting, and declining SHALL leave everything in place. The status bar SHALL say what happened from the receipt, and the page SHALL be listed again.

#### Scenario: From the inbox

Given a focused envelope in the inbox and a trash mailbox, when it is deleted, then nothing asks, the message moves to the trash, the status bar reads `Moved 1 message to the trash`, and the page no longer shows it.

#### Scenario: From the trash

Given two marked envelopes in the trash, when they are deleted, then the TUI asks `Delete 2 messages for good?`, and once confirmed they are removed and the status bar reads `Deleted 2 messages for good`.

#### Scenario: An IMAP server without UIDPLUS

Given a focused envelope in the trash of a server that does not support UIDPLUS, when it is deleted and confirmed, then it is flagged deleted and the status bar says the server expunges it later.

#### Scenario: No trash

Given an account whose backend reports no trash role and no `mailbox.alias.trash`, when an envelope is deleted, then the delete fails with himalaya's error naming `mailbox.alias.trash`, and the envelope stays.

