---
cairn: delta
change: the-tui-runs-on-himalaya
---

## ADDED Requirements

### Requirement: Mail goes through himalaya's commands

(client, a new capability) The TUI SHALL read and change mail only by running himalaya's commands, in-process, over one himalaya session bound to the `-c` paths and the `-a` account: each command an argv the TUI builds, one element per value, parsed by himalaya's own grammar. It SHALL write no protocol or storage logic of its own. It SHALL read each answer as the documented `--json` value of that command, decoded into its own types, never through himalaya's Rust output types, and SHALL claim stdin at startup, so no command reads its input from the terminal.

#### Scenario: A mailbox name holding a space

Given a mailbox named `Old projects`, when its envelopes are listed, then `-m` receives `Old projects` as one argv element and nothing is quoted or split.

#### Scenario: A message is read

Given an envelope on screen, when it is read, then `message read --raw` runs without `--seen`, its bytes are decoded as before, and the message's flags are left as they were.

### Requirement: A partial outcome is a result

(client) A command ending with a partial outcome SHALL be read as a result, its `errors` naming what failed, and only an error carrying no result SHALL be read as a failure. The status bar SHALL say how many items the command acted on and what failed.

#### Scenario: A message another client moved away

Given an envelope on screen whose message another client has since moved, when a flag is added to it, then himalaya answers a count of 0 and one `message-not-found` entry, and the status bar names that failure while the session stays usable.

### Requirement: The connection is kept, and rebuilt in the open

(client) The session SHALL build its client on the first command, keep it, and drop it after a failed command. The commands opening the session SHALL run before the TUI takes the screen. A command that will build the client again mid-session SHALL run with the terminal handed back, as the system editor runs, the screen redrawn after it. Idle for 60 seconds, the TUI SHALL keep the connection alive through himalaya's session keepalive, a failure being logged.

#### Scenario: A credential command prompting at startup

Given a secret command asking for a passphrase on the terminal, when the TUI starts, then the prompt shows on the plain terminal before the interface is drawn.

#### Scenario: A connection the server closed

Given an IMAP connection the server dropped, when a page is listed, then the command fails and says so; the next command runs with the terminal handed back while it connects, and the interface comes back once it answers.

### Requirement: A draft and a sent copy follow the account

(client) A draft SHALL be saved to the account's `drafts` alias, else its drafts role, flagged as a draft. A sent message SHALL be copied where the account's `message.send.save-copy` puts it, as the himalaya CLI copies it. A write a sync engine applies later (pimdir) SHALL say it is queued.

#### Scenario: A JMAP account

Given a JMAP account whose drafts mailbox is named `Brouillons` and carries the drafts role, when a draft is saved, then it lands in `Brouillons`.

### Requirement: The configuration is himalaya's

(configuration) The TUI SHALL resolve its account, the identity it sends as and the signature it appends through himalaya's own configuration, so one file reads the same whichever binary opens it. It SHALL model no backend block. Its own keys (`theme`, `keybinds`, `contact-command`, `contact-complete-key`) SHALL be read beside himalaya's, every key it does not own ignored.

#### Scenario: An option the TUI does not know

Given a file setting an option himalaya added to its `imap` block after the TUI was built, when the TUI starts, then the file loads and the option takes effect, himalaya reading it.

#### Scenario: An identity in the TUI's old spelling

Given an account declaring `from` and `from-name`, when a draft is composed, then it is sent from that address under that name, himalaya taking both as aliases of `email` and `display-name`.

### Requirement: No configuration stops before the screen

(configuration) With no configuration file, or none resolving the account to open, the TUI SHALL fail before taking the screen with himalaya's error naming the path looked at and the documented sample.

#### Scenario: No file at all

Given no configuration file, when the TUI starts, then it prints the path it looked at and the sample's location, and exits without drawing the interface.

## MODIFIED Requirements

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

### Requirement: The account to open is named by `-a`, never positionally

`-a` names the account to open among those the configuration holds, and the default one is opened without it. `-c` takes one path per flag, repeated to merge files, as the himalaya CLI takes it. No positional argument is taken.

#### Scenario: No account is named

Given no `-a`, when the TUI starts, then it opens the account flagged `default = true`.

#### Scenario: A name the file does not carry

Given `-a` naming an account absent from the file, when the TUI starts, then it fails listing the accounts the file does hold.

### Requirement: The signature is assembled from its two keys

The composer SHALL append `signature` introduced by `signature-delim`, both read from himalaya's account merged over the global block, the delimiter defaulting to the RFC 3676 §4.3 `"-- \n"` and written verbatim. The block is therefore exactly the one the himalaya CLI assembles.

#### Scenario: No signature is configured

Given an account and a global block declaring no `signature`, when a draft is composed, then nothing is appended, not even the delimiter.

#### Scenario: A delimiter of one's own

Given `signature = "Regards"` and `signature-delim = "~~~\n"`, when a draft is composed, then the body ends with the delimiter on its own line and the signature under it.

## REMOVED Requirements

### Requirement: A run with no account to open falls back to the CLI's wizard

The wizard leaves with the backend layer it tested connections on; onboarding returns in phase 2 (onboarding-on-the-android-flow).

### Requirement: A configuration file valid for the CLI loads in the TUI

No backend block is modelled here any more: himalaya reads its own, so there is nothing to keep in step.

### Requirement: ALPN comes from the configuration

himalaya's (its config.md and backends.md).

### Requirement: JMAP send targets are configurable

himalaya's.

### Requirement: The server field reads as an authority or a full URL

himalaya's.

### Requirement: A configured path is shell-expanded as the file is read

himalaya's, for its own keys; the TUI's own keys carry no path.

### Requirement: A `unix://` socket carries a pre-authenticated session

himalaya's.

### Requirement: A JMAP blob is fetched from its own authority

himalaya's.

### Requirement: A JMAP mailbox is addressable by name

himalaya's: `-m` resolves aliases, roles and names on every backend.

### Requirement: A credential command is spawned once per account resolution

himalaya's, which resolves the secrets of the client it builds.

### Requirement: The SMTP transport connects on the first send

himalaya's: its unified client leaves SMTP unconnected until a send.

### Requirement: Discovery asks the system resolver first

Discovery leaves with the wizard; phase 2 decides where it lives.
