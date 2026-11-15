---
cairn: delta
change: the-tui-reads-pimdir-draft-13
---

## ADDED Requirements

### Requirement: A pimdir account opens the store himalaya reads

(client) A pimdir account SHALL open a store of the storage draft the pinned himalaya reads, pimdir storage-draft-13, and SHALL refuse a store of an earlier draft with himalaya's own failure, coded `store-version`, shown in the status bar with no mailbox listed.

#### Scenario: A store neverest wrote as draft-11

Given a pimdir account whose store an earlier neverest wrote as storage-draft-11, when the TUI starts, then the status bar shows himalaya's `store-version` refusal and no mailbox is listed, until the store is rebuilt as neverest's MIGRATION.md says.

### Requirement: A message without a subject is listed

(envelopes) An envelope whose `subject` himalaya prints as `null`, a message without a `Subject:` header, SHALL be listed like any other, its subject cell empty.

#### Scenario: A Maildir message without a subject

Given a Maildir inbox holding a message with a `From:` header and no `Subject:`, when its envelopes are listed, then the page holds that envelope, its subject empty.
