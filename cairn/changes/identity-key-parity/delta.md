---
cairn: delta
change: identity-key-parity
---

# Delta

## MODIFIED Requirements

### Requirement: A configuration file valid for the CLI loads in the TUI

Blocks both binaries model (`imap`, `smtp`, `jmap`, `maildir`, `m2dir`, and the `tls` and `sasl` tables inside them) accept every option the CLI accepts there. Options the TUI has no use for are accepted and ignored rather than rejected. Blocks and fields only one binary models are tolerated by the other.

An account's identity SHALL be read under either spelling: `from` and `from-name`, which the TUI writes, and `email` and `display-name`, which the himalaya CLI writes.

#### Scenario: An option the TUI does not act on

Given a configuration file setting `imap.sort.fallback`, which the CLI reads and the TUI does not, when the TUI loads that file, then the file loads and the option has no effect.

#### Scenario: A backend the TUI does not support

Given an account whose block configures `gmail`, when the TUI loads that file, then the block is ignored and the account's other backends stay usable.

#### Scenario: An identity written by the CLI

Given an account declaring `email` and `display-name`, when the TUI opens it, then the composer sends from that address under that name, exactly as if `from` and `from-name` had been used.
