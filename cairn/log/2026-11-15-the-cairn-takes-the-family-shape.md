---
cairn: log
change: the-cairn-takes-the-family-shape
landed: 2026-11-15
---

# The cairn takes the family shape

Tasks X2 and X6 of the family's cli-alignment-2026-11-15 plan (findings F45, F46): housekeeping, no behaviour of the TUI's own moving.

## What landed

- cairn/verify.sh, calendula's verifier copied unchanged; CONTRIBUTING.md names it. It reports the cairn conformant.
- The 14 landed changes still under cairn/changes/ move to cairn/changes/archive/, beside the 6 already there; onboarding-on-the-android-flow, a draft, stays. The 11 relative links of their tasks into spec/ and log/ climb one more level; the log entries keep theirs.
- tests/usage.rs splits into the family's files: tests/exit.rs (a refused command line, a `--json` after `--`, the standard commands) and tests/first_run.rs (a missing configuration, as text and under `--json`), the eight tests unchanged. tests/account.rs has no counterpart: the TUI has no `account` command.
- The CI `features` job of .github/workflows/tests.yml, as himalaya's: clippy `--all-targets -D warnings` over the default and full builds, no backend, `native-tls,jmap`, the local stores together and each backend alone, then the tests of a Maildir-only build.
- pimalaya-config 0.5.2 by `cargo update` (the requirement "0.5" allows it, the lock changing that crate alone): an unknown key's error no longer lists the removed keys among the expected ones.

No capability moved.
