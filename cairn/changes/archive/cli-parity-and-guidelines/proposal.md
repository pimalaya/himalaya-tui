---
cairn: change
id: cli-parity-and-guidelines
status: landed
created: 2026-08-16
---

# Realigned the TUI on the himalaya CLI and on the org guidelines

The TUI and the CLI share one configuration file and one set of per-backend adapters, so the two are meant to read the same file the same way and talk to a server the same way. A file-by-file comparison against the CLI found the TUI trailing on eight points, four of which are wrong behaviour rather than a missing convenience.

The server field is the worst of them. The TUI parses it with a bare `Url::parse`, which reads `mail.example.com:993` as the scheme `mail.example.com` with the path `993`, and accepts any scheme at all. The CLI already carries the fix: a shared `parse_server` that treats a string without `://` as an authority under a default scheme and validates the result against an allowed list. The same helper is what lets `unix:///path` reach a sirup socket, whose greeting is PREAUTH and whose session must therefore negotiate no SASL.

Envelope dates are the second. Chrono validates the optional day-of-week token against the date and rejects the whole timestamp when a sender gets it wrong, which is common enough that the CLI retries without the token. The TUI does not, so those envelopes render with no date.

JMAP blob download is the third. Fastmail serves downloads from a different authority than the API, and reusing the API socket for it sends the request to the API server, which answers with a redirect the non-redirectable download fails on. The CLI opens a fresh authenticated connection when the authorities differ.

JMAP mailbox addressing is the fourth. JMAP mailboxes are opaque ids, and the composer saves a draft to the literal name `Drafts`, which no JMAP account holds. The CLI resolves a name to an id at the shared-client layer, keeping the per-protocol adapters pure id consumers.

The remaining four are alignment rather than repair: the ALPN defaults are duplicated in the configuration instead of being taken from the io- crates that define them, the SMTP transport connects at startup rather than on the first send, discovery pins Cloudflare instead of asking the system resolver first, and the theme module hides its own type behind a private module and a re-export.

The documentation is realigned in the same pass: the sample configuration still points at a retired secret manager and a stale ortie command line, the contributing guide describes a dependency graph that no longer holds, and the README fences its shell blocks without a language.

Two known gaps stay open and are not part of this change, since each is a feature rather than a repair: the wizard still hand-rolls its three discovery probes instead of consuming io-pim-discovery's composed discovery the way the CLI does, and the envelope pager approximates the total with the current page length so the pager always reports a single page.
