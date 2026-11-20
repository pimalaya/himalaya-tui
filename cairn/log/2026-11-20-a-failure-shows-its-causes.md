---
cairn: log
change: a-failure-shows-its-causes
landed: 2026-11-20
---

# A failure shows its causes

Found by the pimdir backend tests of 2026-11-20 (T-A): a pimdir store neverest wrote as storage-draft-13 showed `Error: Open pimdir store <path>` alone, the `store-version` refusal the client spec asks for dropped.

## What landed

- src/tui/update.rs: every failure on the status bar renders its whole chain (`{:#}`): the backend failures, the contact lookup, sending, and the MML compile, template and quote failures, an MML error going through `anyhow::Error` for its sources; the keepalive warning too. Test `a_failure_shows_its_cause_in_the_status_bar`, a pimdir store whose database is text, gated on `unix` and `pimdir`, failing on the old status (the outer context alone).
- Checked by hand over a draft-13 store: the status bar reads the whole `store-version` refusal. The bar is one line, so a narrow terminal still clips a long chain.

## Capabilities moved

- [interface](../spec/interface.md): *A failure shows its causes*, added.
