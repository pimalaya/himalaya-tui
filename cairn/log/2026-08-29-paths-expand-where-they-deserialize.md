---
cairn: log
change: paths-expand-where-they-deserialize
landed: 2026-08-29
---

# A configured path is expanded where it is deserialized

`maildir.root` was a bare `PathBuf` handed to io-maildir exactly as the file wrote it, while config.sample.toml documented it as `~/Mail/example`. Copying the sample got you a literal directory called `~` under the working directory: the account opened, the mailbox listing came back empty, and nothing named the cause. `tls.cert` carried the same gap, an extra root named under a home never being found.

Both now expand on the field. `MaildirConfig::root` goes through `pimalaya_config::toml::shell_expanded_path`, and `TlsConfig::cert` through a private `opt_shell_expanded_path` wrapping it for an optional field, paired with `default` so an absent key still reaches no deserializer. The helper carries a TODO naming the shared variant pimalaya-config does not ship yet. The credential usernames around them already expanded this way, so the schema is now consistent: nothing is expanded at a call site, and no future call site can forget.

The himalaya CLI took the same deserializer on the same keys in the same pass. One file backs both binaries, so a root only one of them expanded would be a root that works in one and not the other.

`downloads-dir` came along as the other half of the audit and is gone from both `Config` and `AccountConfig`. It was declared at both levels and read nowhere: the CLI uses it to place a downloaded attachment, and this binary downloads none. A file setting it still loads, neither struct denying unknown fields, and the test asserting that CLI-only options are accepted keeps the key so the tolerance stays covered. Its two config.sample.toml entries are gone, the sample documenting what this binary reads.

The wizard still expands the folder path typed at its prompt. That is prompt input rather than a configuration field, and it has to resolve to check the directory exists before the account is accepted.

`cairn/spec/configuration.md` gained "A configured path is shell-expanded as the file is read". "A configuration file valid for the CLI loads in the TUI" is unchanged and is what covers a `downloads-dir` the TUI no longer models.
