---
cairn: log
change: a-build-with-no-backend-refuses-to-start
landed: 2026-11-15
---

# A build with no backend refuses to start

`cargo build --no-default-features` failed: src/cli.rs imported `himalaya::account::context::Account`, which himalaya compiles only with a backend or SMTP.

## What landed

- build.rs sets the `backend` cfg when any of the TUI's backend features is on, SMTP included, as himalaya's own build.rs does.
- src/cli.rs: `AccountIdentity` holds what the composer takes from the merged account (signature block, address, name). With a backend it merges the account as before; without one it fails, naming the cargo features.
- src/client.rs: the `..Default::default()` of the session is allowed past `needless_update` in a build where `Session` keeps no other field.
- src/tui/view.rs: the title separates the account with a colon, where it used an em dash.
- Clippy `--all-targets` passes with and without default features.

## Capabilities moved

- [client](../spec/client.md): *A build with no backend refuses to start* added.
