---
cairn: spec
capability: envelopes
---

# Envelopes

How the interface lists a mailbox's messages and moves through them.

### Requirement: The envelope list reaches every message in the mailbox

A listing SHALL be paged, and the page count SHALL come from the total the backend reports for the mailbox rather than from the number of envelopes the current page holds. A backend that cannot report one SHALL fall back to the page length, and only that backend loses the pages it cannot count.

A page SHALL hold exactly the envelopes the panel can show at full height, so that a page is never partly off screen, and SHALL follow the terminal when it is resized, keeping the cursor on the envelope it was on. The panel SHALL name the page it is on, as `(page n/p)`, whenever the mailbox holds more than one.

#### Scenario: A mailbox larger than one page

Given a mailbox holding more messages than the panel can show, when its envelopes are listed, then the page count is the total divided by the rows the panel holds, rounded up, and the panel title reads `(page 1/p)`.

#### Scenario: A page that is exactly full

Given a mailbox holding exactly one page of messages, when its envelopes are listed, then the page count is one and no further page is offered.

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
