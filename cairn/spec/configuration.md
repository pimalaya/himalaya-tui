---
cairn: spec
capability: configuration
---

# Configuration

The TUI reads the himalaya CLI's configuration file, resolving the account, its backends, its identity and its signature through himalaya's own types. It adds a few keys of its own beside them and writes nothing.

### Requirement: The account to open is named by `-a`, never positionally

`-a` names the account to open among those the configuration holds, and the default one is opened without it. `-c` takes one path per flag, repeated to merge files, as the himalaya CLI takes it. No positional argument is taken.

#### Scenario: No account is named

Given no `-a`, when the TUI starts, then it opens the account flagged `default = true`.

#### Scenario: A name the file does not carry

Given `-a` naming an account absent from the file, when the TUI starts, then it fails listing the accounts the file does hold.

### Requirement: The configuration is himalaya's

The TUI SHALL resolve its account, the identity it sends as and the signature it appends through himalaya's own configuration, so one file reads the same whichever binary opens it. It SHALL model no backend block. Its own keys (`theme`, `keybinds`, `contact-command`, `contact-complete-key`) SHALL be read beside himalaya's, every key it does not own ignored.

#### Scenario: An option the TUI does not know

Given a file setting an option himalaya added to its `imap` block after the TUI was built, when the TUI starts, then the file loads and the option takes effect, himalaya reading it.

#### Scenario: An identity in the TUI's old spelling

Given an account declaring `from` and `from-name`, when a draft is composed, then it is sent from that address under that name, himalaya taking both as aliases of `email` and `display-name`.

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
