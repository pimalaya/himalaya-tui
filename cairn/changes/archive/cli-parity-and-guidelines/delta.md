---
cairn: delta
change: cli-parity-and-guidelines
---

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: ALPN comes from the configuration

Each of `imap.alpn`, `smtp.alpn` and `jmap.alpn` sets the ALPN identifiers offered during that protocol's TLS handshake, and is folded into `tls.rustls.alpn`, which the TOML never exposes directly. The default is not restated in the configuration: an omitted list resolves, at connection time, to the one the protocol's own crate defines.

#### Scenario: The option is omitted

Given a block with no `alpn`, when a connection opens, then the protocol default is offered: `["imap"]` for IMAP, `["smtp"]` for SMTP, `["http/1.1"]` for JMAP.

#### Scenario: The option is an empty list

Given `alpn = []`, when a connection opens, then no ALPN negotiation takes place.
