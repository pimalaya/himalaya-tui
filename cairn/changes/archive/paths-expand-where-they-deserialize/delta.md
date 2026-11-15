---
cairn: delta
change: paths-expand-where-they-deserialize
---

## ADDED Requirements

### Requirement: A configured path is shell-expanded as the file is read

Every path the configuration carries (`maildir.root` and the `tls.cert` of each network backend) SHALL be expanded where it is deserialized, not where it is used, so no consumer of the field can be reached by an unexpanded value. Expansion covers a leading `~` and any `$VAR`, and a name that resolves to nothing is kept verbatim rather than failing the load.

The himalaya CLI SHALL expand the same keys the same way, one file backing both binaries.

#### Scenario: A root written the way it is typed in a shell

Given `maildir.root = "~/Mail"`, when the account is opened, then the Maildir under the user's home is read, rather than a literal directory called `~` under the working directory.

#### Scenario: A path field is omitted

Given a `tls` block declaring no `cert`, when the account is opened, then no extra root is trusted and no path is invented.
