---
cairn: spec
capability: configuration
---

# Configuration

The TUI reads the same TOML file as the himalaya CLI, resolved under himalaya/config.toml rather than a name of its own. It only ever reads it: accounts are authored by the CLI's wizard or by hand, and the TUI writes nothing to disk.

### Requirement: The account to open is named by `-a`, never positionally

`-a` addresses the configuration file and nothing else. The positional argument addresses the wizard and nothing else, so neither can stand for the other and the two are mutually exclusive. It is spelled `EMAIL`, an address being what it is reached for, even though the wizard accepts a server URL and a folder path there too.

#### Scenario: No account is named

Given neither `-a` nor a positional argument, when the TUI starts, then it opens the account flagged `default = true`.

#### Scenario: A name the file does not carry

Given `-a` naming an account absent from the file, when the TUI starts, then it fails listing the accounts the file does hold, and no wizard runs.

#### Scenario: A name and no file at all

Given `-a` and no configuration file at the resolved path, when the TUI starts, then it fails naming the path, and no wizard runs.

#### Scenario: Both are given

Given `-a` and a positional argument together, when the TUI starts, then it fails on the conflict before anything is loaded.

### Requirement: A run with no account to open falls back to an in-memory one

A session needs an account to open. When the configuration resolves none, a wizard builds one that lives for that session alone, proposing nothing for the configuration file.

#### Scenario: The wizard is seeded

Given a positional argument, when the TUI starts, then the account lookup is skipped, the wizard discovers from that value without prompting for it, and no warning is raised. The rest of the file still applies.

#### Scenario: The file is refused outright

Given `--no-config`, when the TUI starts, then the file is not read at all, the wizard prompts for its input, and no warning is raised.

#### Scenario: No configuration file

Given no file at the default paths nor at the one `-c` names, and no seed, when the TUI starts, then it warns naming the path it looked for and runs the wizard.

#### Scenario: A file carrying no default account

Given a configuration file whose accounts none flags `default`, when the TUI starts with no account named and no seed, then it warns naming what is missing and runs the wizard.

#### Scenario: The account the wizard produced

Given an account the wizard built, when the session ends, then nothing was written to disk and the configuration file is unchanged.

### Requirement: A configuration file valid for the CLI loads in the TUI

Blocks both binaries model (`imap`, `smtp`, `jmap`, `maildir`, `m2dir`, and the `tls` and `sasl` tables inside them) accept every option the CLI accepts there. Options the TUI has no use for are accepted and ignored rather than rejected. Blocks and fields only one binary models are tolerated by the other.

#### Scenario: An option the TUI does not act on

Given a configuration file setting `imap.sort.fallback`, which the CLI reads and the TUI does not, when the TUI loads that file, then the file loads and the option has no effect.

#### Scenario: A backend the TUI does not support

Given an account whose block configures `gmail`, when the TUI loads that file, then the block is ignored and the account's other backends stay usable.

### Requirement: ALPN comes from the configuration

Each of `imap.alpn`, `smtp.alpn` and `jmap.alpn` sets the ALPN identifiers offered during that protocol's TLS handshake, and is folded into `tls.rustls.alpn`, which the TOML never exposes directly. The default is not restated in the configuration: an omitted list resolves, at connection time, to the one the protocol's own crate defines.

#### Scenario: The option is omitted

Given a block with no `alpn`, when a connection opens, then the protocol default is offered: `["imap"]` for IMAP, `["smtp"]` for SMTP, `["http/1.1"]` for JMAP.

#### Scenario: The option is an empty list

Given `alpn = []`, when a connection opens, then no ALPN negotiation takes place.

### Requirement: JMAP send targets are configurable

`jmap.identity-id` names the sending identity and `jmap.drafts-mailbox-id` the mailbox a message is staged in before submission.

#### Scenario: Either is omitted

Given no configured id, when a message is sent, then the identity is the first one `Identity/get` reports and the mailbox is the one whose role is `drafts`, and the send fails when the account exposes neither.

### Requirement: The server field reads as an authority or a full URL

Each of `imap.server`, `smtp.server` and `jmap.server` accepts a full `scheme://` URL used verbatim, or a string carrying no `://` at all, which is read as an authority under that protocol's secure default scheme (`imaps`, `smtps`, `https`). The resulting scheme is validated: IMAP accepts `imap`, `imaps` and `unix`, SMTP accepts `smtp`, `smtps` and `unix`, JMAP accepts `http` and `https`.

#### Scenario: A bare host and port

Given `imap.server = "mail.example.com:993"`, when the connection opens, then the host is `mail.example.com` and the port is 993, rather than the whole string being read as a scheme.

#### Scenario: A scheme the protocol does not speak

Given `imap.server = "ftp://mail.example.com"`, when the connection opens, then it fails naming the scheme and the accepted ones.

### Requirement: A `unix://` socket carries a pre-authenticated session

An IMAP or SMTP server given as `unix:///path` reaches a local socket proxy such as sirup, whose greeting is already authenticated.

#### Scenario: SASL is configured anyway

Given a `unix://` server and a `sasl` block, when the session opens, then no authentication is negotiated and the configured mechanism is ignored.

### Requirement: A JMAP blob is fetched from its own authority

The session, upload and download URLs a JMAP session advertises may live on different hosts.

#### Scenario: The download host differs from the API host

Given a session whose download URL resolves to another authority than the API URL, when a message is read, then a fresh authenticated connection is opened to the download host rather than the API socket being reused.

### Requirement: A JMAP mailbox is addressable by name

JMAP mailboxes carry opaque ids, while the interface addresses a mailbox by the name a user reads.

#### Scenario: A name that is not an id

Given the composer saving a draft to `Drafts`, when the active backend is JMAP, then the name is mapped to the id of the mailbox carrying it, from a mailbox listing fetched once per session.

#### Scenario: A value that already is an id

Given a mailbox argument matching a known id, when it is resolved, then it passes through unchanged.

### Requirement: The SMTP transport connects on the first send

An account configuring SMTP alongside a storage backend opens no SMTP connection until a message is actually sent.

#### Scenario: A session that never sends

Given an account with both an `imap` and an `smtp` block, when the session runs without sending, then no SMTP connection is opened.

#### Scenario: The transport cannot connect

Given an unreachable SMTP server, when a message is sent, then the send fails naming the connection error, and the rest of the session stays usable.

### Requirement: Discovery asks the system resolver first

The wizard resolves DNS through the host's own resolver, so a corporate or split-horizon network resolves the way every other program on that host does.

#### Scenario: No system resolver is available

Given a host exposing no usable resolver and no `HIMALAYA_DNS_RESOLVER`, when discovery runs, then it falls back to a public DNS-over-TCP resolver.
