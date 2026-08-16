---
cairn: log
change: bump-pim-libs
landed: 2026-08-16
---

# Bumped every Pimalaya dependency to its latest release

io-imap went 0.3 to 0.5, io-smtp 0.2 to 0.3, io-jmap 0.2 to 0.3, io-pim-discovery 0.3 to 0.7, pimalaya-cli 0.1 to 0.2 and pimalaya-stream 0.1 to 0.3; io-m2dir, io-maildir and pimalaya-config moved to their latest patch releases. MSRV rose to 1.89, which pimalaya-config 0.1.4 requires.

SASL moved out of pimalaya-stream into the new io-sasl dependency: `SaslConfig::try_into_sasl` now builds `SaslPlainCreds` and its siblings, and SCRAM-SHA-256 carries an empty nonce for the client to fill plus `SaslGs2ChannelBinding::Unsupported`. `ImapClient::new` and `SmtpClient::new` pass `ImapSessionOpenOptions` and `SmtpSessionOpenOptions` in place of loose `starttls` and `auto_id` arguments, and the IMAP and SMTP adapters import the `ImapClient` and `SmtpClient` traits the commands now live behind. `STATUS` takes a `Cow` of item names.

mime-meta-language is pinned to the mml git repository. The composer was written against the unreleased mml API and the crate did not build against the published 1.1.1 at all, so this restores a working build rather than moving forward from one; the pin comes out when mml releases.

No capability files moved: the spec is still empty, and this change adds no requirement to it. The behaviour that did shift comes from the libraries, a retried rather than failed not-ready socket and a per-exchange SCRAM nonce, neither of which the TUI states as a requirement of its own.
