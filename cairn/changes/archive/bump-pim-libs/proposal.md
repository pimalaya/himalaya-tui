---
cairn: change
id: bump-pim-libs
status: landed
created: 2026-08-16
---

# Bump every Pimalaya dependency to its latest release

The TUI had drifted behind the libraries it is built on: io-imap 0.3 against a published 0.5, io-smtp 0.2 against 0.3, io-jmap 0.2 against 0.3, io-pim-discovery 0.3 against 0.7, pimalaya-cli 0.1 against 0.2 and pimalaya-stream 0.1 against 0.3. Each of those carries breaking changes the TUI never absorbed, so the drift only grows and the eventual migration only gets harder.

The composer had drifted the other way: it was written against the unreleased mml API (`MmlCompileOptions`, template builders taking a rendered quote), which the pinned mime-meta-language 1.1.1 does not carry, so the crate did not build at all against a released dependency set.

## What changes

Every Pimalaya dependency moves to its latest published version, and the call sites move with it:

- SASL leaves pimalaya-stream for io-sasl, whose credential structs carry a `Creds` suffix and whose SCRAM credentials gained a client nonce and a channel binding.
- `ImapClientStd::connect` and `SmtpClientStd::connect` take an options struct in place of loose arguments, and the forty-odd IMAP commands moved behind the `ImapClient` trait, likewise `send` behind `SmtpClient`.
- `Cargo.toml` pins mime-meta-language to the mml git repository until mml ships a release carrying the composer API, since a pin on 1.1.1 cannot build.
- MSRV moves to 1.89, which pimalaya-config 0.1.4 requires and which the CLI already declares.

No user-facing behaviour is designed to change. What does change comes from the libraries: pimalaya-stream retries a not-ready socket for up to a minute instead of failing on the first `EAGAIN`, and SCRAM-SHA-256 now runs with a client nonce drawn per exchange.
