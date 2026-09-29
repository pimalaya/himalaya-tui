---
cairn: spec
capability: contacts
---

# Contacts

The TUI holds no address book. Recipients complete from a command the user configures, the aerc `address-book-cmd` way, so any contact tool plugs in.

### Requirement: Recipients complete from a contact command

The TUI SHALL hold no address book of its own. `contact-command`, global or per account with the account winning, is run with the typed fragment and its stdout read as one contact per line, the address then an optional tab and name. A `%s` is replaced by the fragment, shell-quoted in a shell line; with no `%s`, the fragment is appended as a last argument.

#### Scenario: A shell line with a placeholder

Given `contact-command = "khard email --parsable %s"` and the fragment `o'brien`, when completion runs, then the shell receives `khard email --parsable 'o'\''brien'`.

#### Scenario: No command configured

Given no `contact-command`, when the complete key is pressed in a recipient header, then the status bar says no contact command is configured and the buffer is untouched.

### Requirement: Completion only acts on recipient addresses

The complete key (`contact-complete-key`, `tab` by default) SHALL complete only in insert mode, in the address part of a `To`, `Cc`, `Bcc` or `Reply-To` header or one of its continuation lines, before the blank line ending the headers. Anywhere else it SHALL reach the editor unchanged. The fragment is the text after the last comma, or after the colon, up to the cursor.

#### Scenario: Tab in the body

Given the cursor in the message body, when Tab is pressed, then the editor inserts it as before.

#### Scenario: Second recipient

Given `To: alice@example.com, bo` with the cursor at the end, when Tab is pressed, then the command is queried with `bo` and only `bo` is replaced.

### Requirement: The lookup never blocks the interface

The command SHALL run off the event loop and be killed after five seconds. A result SHALL be dropped when the buffer no longer holds the queried fragment at the queried place. No match and failures report on the status bar, one match is inserted directly, several open a list at the cursor.

#### Scenario: One match

Given a command answering one line `bob@example.com\tBob Smith`, when completion runs on `bo`, then `bo` becomes `Bob Smith <bob@example.com>`.

#### Scenario: A name carrying a comma

Given the answer `doe@example.com\tDoe, John`, when it is inserted, then it reads `"Doe, John" <doe@example.com>`.
