---
cairn: change
id: onboarding-on-the-android-flow
status: draft
created: 2026-11-14
---

# Onboarding on the Android flow

> Phase 2 of the plan of 2026-11-14, after the-tui-runs-on-himalaya. A sketch: the design is settled when phase 1 has landed.

## Why

After phase 1, a run with no configuration stops with himalaya's error and the sample's location. The family dropped its CLI wizards (decided 2026-11-08, himalaya the-wizard-leaves on 2026-11-11), so nothing in the mail stack sets up an account any more. The TUI is the mail frontend a person meets first, and the onboarding the owner wants everywhere is the Android app's: the flow the four CLI wizards adopted on 2026-11-08 before they left.

## What

- **When it runs.** When the configuration resolves no account to open, in the TUI's own screens rather than terminal prompts. An entry point to add an account to an existing file is decided with the design.
- **The flow.** An address, then automatic discovery and a pick among the services found, then a review of what will be written. A password is typed once and stored in the system keyring, or OAuth 2.0 is handed to Ortie. A server URL is asked only when nothing was found.
- **Checking before writing.** The candidate account goes into a throw-away configuration file and himalaya's `account check` runs on it, in-process, through a session bound to that file. The file names the keyring command and never holds the password, so removing it afterwards is housekeeping, not secrecy.
- **Writing.** The account is appended to the real file only once the check passes, under a name no other account takes, comments and formatting untouched. The session then opens on it.

## Open questions

- **Discovery.** Where it lives: io-pim-discovery straight in the TUI, himalaya having none since the-wizard-leaves. The CLI wizards' rule holds: never its rfc9728 or rfc8414 lookups, which ignore the discovery deadline, and plain HTTP only on a loopback host.
- **The keyring.** Which store per system, and how the write is done (the tools the sample documents: secret-tool, security, keyctl, cmdkey, pass).
- **The Ortie hand-off** on Ortie 3, which removed `ortie configure`.
- **Re-entry.** `--no-config`, a seeded address, an in-TUI "add account": which come back.
- **Shared flow.** What is shared with the Android app and the other frontends, and what stays a TUI screen.
