---
cairn: spec
capability: configuration
---

# Configuration

The TUI reads the himalaya CLI's configuration file, resolving the account, its backends, its identity and its signature through himalaya's own types. It adds a few keys of its own under `tui` tables and writes nothing.

### Requirement: The account to open is named by `-a`, never positionally

`-a` names the account to open among those the configuration holds, and the default one is opened without it. `-c` takes one path per flag, repeated to merge files, as the himalaya CLI takes it. No positional argument is taken.

#### Scenario: No account is named

Given no `-a`, when the TUI starts, then it opens the account flagged `default = true`.

#### Scenario: A name the file does not carry

Given `-a` naming an account absent from the file, when the TUI starts, then it fails listing the accounts the file does hold.

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

### Requirement: No configuration stops before the screen

With no configuration file, or none resolving the account to open, the TUI SHALL fail before taking the screen with himalaya's error naming the path looked at and the documented sample.

#### Scenario: No file at all

Given no configuration file, when the TUI starts, then it prints the path it looked at and the sample's location, and exits without drawing the interface.

### Requirement: The signature is assembled from its two keys

The composer SHALL append `signature` introduced by `signature-delim`, both read from himalaya's account merged over the global block, the delimiter defaulting to the RFC 3676 §4.3 `"-- \n"` and written verbatim. The block is therefore exactly the one the himalaya CLI assembles.

#### Scenario: No signature is configured

Given an account and a global block declaring no `signature`, when a draft is composed, then nothing is appended, not even the delimiter.

#### Scenario: A delimiter of one's own

Given `signature = "Regards"` and `signature-delim = "~~~\n"`, when a draft is composed, then the body ends with the delimiter on its own line and the signature under it.
