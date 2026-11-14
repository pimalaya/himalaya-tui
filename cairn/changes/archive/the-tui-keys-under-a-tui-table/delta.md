---
cairn: delta
change: the-tui-keys-under-a-tui-table
---

## MODIFIED Requirements

### Requirement: The configuration is himalaya's

The TUI SHALL resolve its account, the identity it sends as and the signature it appends through himalaya's own configuration, so one file reads the same whichever binary opens it. It SHALL model no backend block. Its own keys SHALL live under a `tui` table, the only foreign table himalaya loads: `keybinds`, `contact-command`, `contact-complete-key` and `theme` under the top-level `[tui]`, an account's `contact-command` under `[accounts.<name>.tui]`. The `tui` tables SHALL refuse a key they do not know, naming it; every key outside them SHALL be ignored by the TUI, being himalaya's. The TUI SHALL NOT read its keys anywhere else.

#### Scenario: An option the TUI does not know

Given a file setting an option himalaya added to its `imap` block after the TUI was built, when the TUI starts, then the file loads and the option takes effect, himalaya reading it.

#### Scenario: An identity in the TUI's old spelling

Given an account declaring `from` and `from-name`, when a draft is composed, then it is sent from that address under that name, himalaya taking both as aliases of `email` and `display-name`.

#### Scenario: The TUI keys under the tui tables

Given `[tui]` setting `keybinds`, `contact-command`, `contact-complete-key` and `theme`, and `[accounts.work.tui]` setting `contact-command`, when the TUI starts on `work`, then each takes effect, the account's command winning over the top-level one, and himalaya loads the same file.

#### Scenario: A typo under tui

Given `[tui]` setting `keybind = "emacs"`, when the TUI starts, then it fails naming the unknown field `keybind`.

#### Scenario: A TUI key outside the tui tables

Given `keybinds` at the top level or `contact-command` in an account, when the TUI starts, then the file is refused with himalaya's error naming the unknown key, and the TUI would not have read it either.

### Requirement: Recipients complete from a contact command

The TUI SHALL hold no address book of its own. `contact-command`, under `[tui]` or `[accounts.<name>.tui]` with the account winning, is run with the typed fragment and its stdout read as one contact per line, the address then an optional tab and name. A `%s` is replaced by the fragment, shell-quoted in a shell line; with no `%s`, the fragment is appended as a last argument.

#### Scenario: A shell line with a placeholder

Given `tui.contact-command = "khard email --parsable %s"` and the fragment `o'brien`, when completion runs, then the shell receives `khard email --parsable 'o'\''brien'`.

#### Scenario: No command configured

Given no `contact-command`, when the complete key is pressed in a recipient header, then the status bar says no contact command is configured and the buffer is untouched.
