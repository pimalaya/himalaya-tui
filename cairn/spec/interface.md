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
