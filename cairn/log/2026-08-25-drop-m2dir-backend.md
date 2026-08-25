---
cairn: log
change: drop-m2dir-backend
landed: 2026-08-25
---

# Dropped the m2dir backend before the first release

m2dir may retire, pimdir succeeds it as the local store a sync engine fills, and Maildir stays for the users whose mail already lives in one. Shipping a first release with a third local backend already on its way out would have bought a deprecation to run later, so it goes now, while it has no user to migrate.

The `m2dir` cargo feature, the io-m2dir dependency and src/m2dir/ are gone, with the `BackendClient::M2dir` variant and its eight match arms, the `M2dirConfig` block, its `RENDER_ORDER` entry and the wizard's check arm. `AccountConfig` denies no unknown field, so a shared configuration file still carrying an `[accounts.<name>.m2dir]` block loads as before and the account's other backends stay usable: the block is now ignored the way a `gmail` block already was, which is the existing parity requirement rather than a new one.

src/wizard/local.rs went with it. The module existed to choose between the two local backends, detecting m2dir from a `.m2store` or `.m2dir` marker and Maildir from its `cur`/`new`/`tmp` tree, then prompting when neither answered and both were compiled in. With one local backend left there is nothing to detect and nothing to pick, and a maildir-only build already resolved to Maildir whatever the markers said, so `discover::configure_local` builds the `MaildirConfig` itself and the `Local` enum, `detect` and the three `pick` variants are gone.

The configuration capability moved: the list of blocks both binaries model no longer names `m2dir`, and the scenario for a backend the TUI does not support now names it beside `gmail`. Nothing else in the spec moved, and the CHANGELOG's backend bullet drops it rather than gaining a removal note, the release it belongs to being the first.
