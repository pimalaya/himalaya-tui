---
cairn: change
id: drop-m2dir-backend
status: landed
created: 2026-08-25
---

# Drop the m2dir backend before the first release

m2dir may retire. pimdir is its successor as the local store the sync engine fills, and Maildir stays for the users whose mail already lives in one. Shipping a first release with three local backends, one of them on its way out, buys the project a deprecation to run later and buys its users a choice that will not survive.

The backend also costs more than it looks. It is the only one reading a whole mailbox and parsing every message to answer a listing, it surfaces no counts, and it has no native copy or move, so both are a read followed by a write. None of that is worth carrying for a format that has no user yet: the crate has no released consumer, the wizard offers it only when a folder happens to carry a `.m2store` or `.m2dir` marker, and nothing in the configuration sample documents it.

## What changes

The `m2dir` cargo feature, the io-m2dir dependency and src/m2dir/ go, along with the `BackendClient::M2dir` variant, the `M2dirConfig` block and the wizard's local detection and pick list, which drops back to Maildir alone.

A configuration file still carrying an `[accounts.<name>.m2dir]` block keeps loading: `AccountConfig` does not deny unknown fields, so the block is ignored the way a `gmail` block already is, and the account's other backends stay usable. That is the existing parity requirement rather than a new one, and it is why this removal needs no migration.

Nothing else moves. The `maildir` feature, its adapter and its tests stay exactly as they are, and `pimdir` is not added here: this change only stops shipping m2dir.
