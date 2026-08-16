---
cairn: spec
capability: configuration
---

# Configuration

The TUI reads the same TOML file as the himalaya CLI, resolved under `himalaya/config.toml` rather than a name of its own.

### Requirement: A configuration file valid for the CLI loads in the TUI

Blocks both binaries model (`imap`, `smtp`, `jmap`, `maildir`, `m2dir`, and the `tls` and `sasl` tables inside them) accept every option the CLI accepts there. Options the TUI has no use for are accepted and ignored rather than rejected. Blocks and fields only one binary models are tolerated by the other.

#### Scenario: An option the TUI does not act on

Given a configuration file setting `imap.sort.fallback`, which the CLI reads and the TUI does not, when the TUI loads that file, then the file loads and the option has no effect.

#### Scenario: A backend the TUI does not support

Given an account whose block configures `gmail`, when the TUI loads that file, then the block is ignored and the account's other backends stay usable.

### Requirement: ALPN comes from the configuration

Each of `imap.alpn`, `smtp.alpn` and `jmap.alpn` sets the ALPN identifiers offered during that protocol's TLS handshake, and is folded into `tls.rustls.alpn`, which the TOML never exposes directly.

#### Scenario: The option is omitted

Given a block with no `alpn`, when a connection opens, then the protocol default is offered: `["imap"]` for IMAP, `["smtp"]` for SMTP, `["http/1.1"]` for JMAP.

#### Scenario: The option is an empty list

Given `alpn = []`, when a connection opens, then no ALPN negotiation takes place.

### Requirement: JMAP send targets are configurable

`jmap.identity-id` names the sending identity and `jmap.drafts-mailbox-id` the mailbox a message is staged in before submission.

#### Scenario: Either is omitted

Given no configured id, when a message is sent, then the identity is the first one `Identity/get` reports and the mailbox is the one whose role is `drafts`, and the send fails when the account exposes neither.
