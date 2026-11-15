---
cairn: delta
change: several-messages-at-once
---

## ADDED Requirements

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

## MODIFIED Requirements

## REMOVED Requirements
