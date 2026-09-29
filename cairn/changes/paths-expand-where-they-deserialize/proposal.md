---
cairn: change
id: paths-expand-where-they-deserialize
status: landed
created: 2026-08-29
---

# Expand a configured path where it is deserialized, not where it is read

`maildir.root` is declared as a bare `PathBuf` and handed to io-maildir as it was written. The sample documents it as `~/Mail/example`, the wizard writes an expanded absolute path, and the two disagree: a root a user copies out of the sample opens a literal directory called `~` under the working directory, so every mailbox listing comes back empty and nothing says why. `tls.cert` has the same shape and the same gap, an extra root a user names under their home never being found.

The rest of the schema already does this correctly, and does it at the right place: every credential username goes through `pimalaya_config::toml::shell_expanded_string` on the field. A path expanded at a call site is only expanded for the call sites that remembered, which is how this got missed in the first place.

`downloads-dir` is separate and comes along because it is the other half of the same audit. It is declared at both levels and read nowhere: the himalaya CLI uses it to place a downloaded attachment, and this binary downloads none. Blocks and fields only one binary models are already tolerated by the other, so dropping the field costs no file its load.

## What changes

`MaildirConfig::root` takes `#[serde(deserialize_with = "shell_expanded_path")]`, and `TlsConfig::cert` takes `#[serde(default, deserialize_with = "opt_shell_expanded_path")]`, a private helper wrapping the shared one for an optional field until pimalaya-config ships that variant itself. The himalaya CLI moves onto the same deserializer for the same keys, one file backing both binaries.

`Config::downloads_dir` and `AccountConfig::downloads_dir` are removed, with the two sample entries documenting them. A file setting the key still loads: neither struct denies unknown fields.

The wizard keeps expanding the folder path typed at its prompt, which is prompt input rather than a configuration field: it has to resolve the path to check the directory is there before accepting the account.
