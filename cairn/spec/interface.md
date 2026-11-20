---
cairn: spec
capability: interface
---

# Interface

How the event loop keeps the screen honest about what it is doing.

### Requirement: A blocking action is announced before it runs

An action that blocks the loop on the backend (loading envelopes, reading, replying to or forwarding a message, copying, moving, flagging, sending, saving a draft) SHALL first set its status and let a frame draw it, then run on the next iteration, then draw its outcome without waiting for a key. The announcing status SHALL not outlive the action: an outcome setting no status of its own leaves the status bar empty.

#### Scenario: Sending

Given a composed message, when it is sent, then `Sending message…` is on screen while the SMTP transaction runs, and `Message sent` replaces it once it returns.

### Requirement: A failure shows its causes

A failure SHALL be shown on the status bar with its whole chain of causes, each joined to the step it broke by `: `, never the outermost step alone.

#### Scenario: A store that is not pimdir's

Given a pimdir account whose `pimdir.db` is not an SQLite database, when the TUI starts, then the status bar reads `Error: Open pimdir store` followed by the store path and the cause saying the file is not a database, and no mailbox is listed.
