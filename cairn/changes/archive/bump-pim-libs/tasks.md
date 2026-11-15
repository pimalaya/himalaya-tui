---
cairn: tasks
change: bump-pim-libs
---

- [x] Bump io-imap, io-jmap, io-smtp, io-pim-discovery, pimalaya-cli and pimalaya-stream to their latest published majors.
- [x] Refresh the lockfile so io-m2dir, io-maildir and pimalaya-config sit on their latest patch releases.
- [x] Add io-sasl and move `SaslConfig::try_into_sasl` onto its `*Creds` structs, drawing the SCRAM nonce per exchange and declaring the channel binding unsupported.
- [x] Move `ImapClient::new` onto `ImapSessionOpenOptions` and `SmtpClient::new` onto `SmtpSessionOpenOptions`.
- [x] Import `io_imap::client::ImapClient` and `io_smtp::client::SmtpClient` where the commands are called.
- [x] Pin mime-meta-language to the mml git repository so the composer builds.
- [x] Raise MSRV to 1.89 for pimalaya-config 0.1.4.
- [x] Check every backend feature combination, then run fmt, clippy and the test suite.
- [ ] Drop the mime-meta-language patch once mml publishes the composer API.
