---
cairn: spec
capability: client
---

# Client

How the interface reads and changes mail: through himalaya's own commands, run in-process over one session.

### Requirement: Mail goes through himalaya's commands

The TUI SHALL read and change mail only by running himalaya's commands, in-process, over one himalaya session bound to the `-c` paths and the `-a` account: each command an argv the TUI builds, one element per value, parsed by himalaya's own grammar. It SHALL write no protocol or storage logic of its own. It SHALL read each answer as the documented `--json` value of that command, decoded into its own types, never through himalaya's Rust output types, and SHALL claim stdin at startup, so no command reads its input from the terminal.

#### Scenario: A mailbox name holding a space

Given a mailbox named `Old projects`, when its envelopes are listed, then `-m` receives `Old projects` as one argv element and nothing is quoted or split.

#### Scenario: A message is read

Given an envelope on screen, when it is read, then `message read --raw` runs without `--seen`, its bytes are decoded as before, and the message's flags are left as they were.

### Requirement: A partial outcome is a result

A command ending with a partial outcome SHALL be read as a result, its `errors` naming what failed, and only an error carrying no result SHALL be read as a failure. The status bar SHALL say how many items the command acted on and what failed.

#### Scenario: A message another client moved away

Given an envelope on screen whose message another client has since moved, when a flag is added to it, then himalaya answers a count of 0 and one `message-not-found` entry, and the status bar names that failure while the session stays usable.

### Requirement: The connection is kept, and rebuilt in the open

The session SHALL build its client on the first command, keep it, and drop it after a failed command. The commands opening the session SHALL run before the TUI takes the screen. A command that will build the client again mid-session SHALL run with the terminal handed back, as the system editor runs, the screen redrawn after it. Idle for 60 seconds, the TUI SHALL keep the connection alive through himalaya's session keepalive, a failure being logged.

#### Scenario: A credential command prompting at startup

Given a secret command asking for a passphrase on the terminal, when the TUI starts, then the prompt shows on the plain terminal before the interface is drawn.

#### Scenario: A connection the server closed

Given an IMAP connection the server dropped, when a page is listed, then the command fails and says so; the next command runs with the terminal handed back while it connects, and the interface comes back once it answers.

### Requirement: A draft and a sent copy follow the account

A draft SHALL be saved to the account's `drafts` alias, else its drafts role, flagged as a draft. A sent message SHALL be copied where the account's `message.send.save-copy` puts it, as the himalaya CLI copies it. A write a sync engine applies later (pimdir) SHALL say it is queued.

#### Scenario: A JMAP account

Given a JMAP account whose drafts mailbox is named `Brouillons` and carries the drafts role, when a draft is saved, then it lands in `Brouillons`.
