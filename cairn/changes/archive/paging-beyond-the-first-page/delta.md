---
cairn: delta
change: paging-beyond-the-first-page
---

## ADDED Requirements

### Requirement: The envelope list reaches every message in the mailbox

A listing SHALL be paged, and the page count SHALL come from the total the backend reports for the mailbox rather than from the number of envelopes the current page holds. A backend that cannot report one SHALL fall back to the page length, and only that backend loses the pages it cannot count.

The interface SHALL name the page it is on whenever the mailbox holds more than one.

#### Scenario: A mailbox larger than one page

Given a mailbox holding more messages than the page size, when its envelopes are listed, then the page count is the total divided by the page size, rounded up, and the envelopes panel names the current page beside it.

#### Scenario: A page that is exactly full

Given a mailbox holding exactly one page of messages, when its envelopes are listed, then the page count is one and no further page is offered.

#### Scenario: Paging past the last page

Given the last page of a mailbox, when the next page is asked for, then nothing moves and the page already shown stays.

### Requirement: Navigation crosses the page boundary

Moving past the end of a page SHALL load the next one and select its first envelope, and moving before the start of a page SHALL load the previous one and select its last. The paging keys SHALL land on the first envelope of the page they reach, whichever direction they moved in.

#### Scenario: Moving down from the last envelope of a page

Given the last envelope of a page that is not the last, when the next envelope is asked for, then the following page is loaded and its first envelope is selected.

#### Scenario: Moving up from the first envelope of a page

Given the first envelope of a page that is not the first, when the previous envelope is asked for, then the preceding page is loaded and its last envelope is selected.

#### Scenario: Moving down from the last envelope of the last page

Given the last envelope of the last page, when the next envelope is asked for, then the selection does not move.
