---
cairn: log
change: cli-parity-and-guidelines
landed: 2026-08-16
---

# Realigned the TUI on the himalaya CLI and on the org guidelines

A file-by-file comparison against the CLI drove this change. Four of the eight gaps it found were wrong behaviour.

The three permissive server parsers are gone, replaced by the CLI's `parse_server` in config.rs plus a per-protocol wrapper next to each client. A string carrying no `://` is now read as an authority under the protocol's secure default scheme, so `mail.example.com:993` no longer parses as the scheme `mail.example.com` with the path `993`, and the resulting scheme is validated against a per-protocol list. That list carries `unix` for IMAP and SMTP: a `unix://` server reaches a sirup socket, whose greeting is PREAUTH, so the session negotiates no SASL even when a `sasl` block is configured. A server URL yielding no host now fails instead of authenticating against an empty one.

IMAP envelope dates retry without their day-of-week token when chrono rejects the pair, so a sender that gets the weekday wrong no longer loses its date entirely.

JMAP blob download opens a fresh authenticated connection when the download URL's authority differs from the API's, which is what reading a message on Fastmail needs. And a mailbox name now resolves to its opaque JMAP id at the shared client, before dispatch, so the composer's `Drafts` target lands on a JMAP account; the per-protocol adapters stay pure id consumers.

The four alignment items: the ALPN defaults come from io-imap, io-smtp and io-jmap rather than being restated (`alpn` is now `Option<Vec<String>>`, an omitted list resolving at connect time and an explicit `[]` still skipping negotiation); the SMTP transport is a three-state slot connected on the first send; discovery reads `HIMALAYA_DNS_RESOLVER`, then the system resolver, then Cloudflare; and `Theme` moved from a private module behind a re-export into the sibling src/tui/theme.rs that declares the presets.

Along the way the configuration module dropped every cargo-feature gate that pulled no crate, keeping only `allow(dead_code)` where a partial build leaves a converter without a caller, and the same treatment on `Flag` fixed an m2dir-only build that had been broken: `flags_to_m2dir` calls `Flag::iana`, whose gate did not list m2dir. Three unused dependencies (uuid, serde_json, shellexpand) are out and io-pim-discovery is back in alphabetical order. Clippy is clean on all-features and on five reduced feature sets.

The documentation moved with it: the sample configuration lost its retired secret-manager and stale ortie command lines and gained the `unix://` servers, the README fences its shell blocks and points at the CLI's provider recipes rather than restating them, and the contributing guide no longer claims an empty patch table or SASL living in pimalaya-stream.

The capability file cairn/spec/configuration.md gained five requirements and its ALPN one was rewritten to say where the default comes from. Two known gaps stay open and were deliberately left out, each being a feature rather than a repair: the wizard still hand-rolls its three discovery probes instead of consuming io-pim-discovery's composed discovery, and the envelope pager approximates the total with the current page length so it always reports a single page.
