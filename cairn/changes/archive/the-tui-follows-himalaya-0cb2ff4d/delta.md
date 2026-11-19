---
cairn: delta
change: the-tui-follows-himalaya-0cb2ff4d
---

## MODIFIED Requirements

### Requirement: A draft and a sent copy follow the account

(client) A draft SHALL be saved to the account's `drafts` alias, else its drafts role, flagged as a draft. A sent message SHALL be copied where the account's `message.send.save-copy` puts it, as the himalaya CLI copies it. A write a sync engine applies later (pimdir) SHALL say it is queued. A send whose receipt names an error, a copy failing after the message went out, SHALL say the message was sent and name the first error, and SHALL close the composer, as sending again would send the message twice.

#### Scenario: A JMAP account

Given a JMAP account whose drafts mailbox is named `Brouillons` and carries the drafts role, when a draft is saved, then it lands in `Brouillons`.

#### Scenario: A copy to a mailbox the account does not hold

Given an account whose `message.send.save-copy` names a mailbox the backend does not hold, when a message is sent, then the status bar reads `Message sent, error: ` followed by himalaya's failure naming that mailbox, and the composer is closed.

### Requirement: A pimdir account opens the store himalaya reads

(client) A pimdir account SHALL open a store of the storage draft the pinned himalaya reads, pimdir storage-draft-14, and SHALL refuse a store of another format version with himalaya's own failure, coded `store-version`, shown in the status bar with no mailbox listed.

#### Scenario: A store neverest wrote as draft-13

Given a pimdir account whose store an earlier neverest wrote as storage-draft-13, when the TUI starts, then the status bar shows himalaya's `store-version` refusal and no mailbox is listed, until the store is synced with that neverest until nothing is queued, moved aside and synced anew.
