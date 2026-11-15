---
cairn: log
change: the-account-failure-is-coded
landed: 2026-11-15
---

# The account failure is coded

Owner rulings of 2026-11-15, items 16 and 24 of the merged list, through himalaya 6f7c7a1f.

## What landed

- src/main.rs: the failure to build the first screen goes through `himalaya::error::coded` and is reported with himalaya's `ErrorCode`, so a name no account has, no or several defaults, and a backend the build leaves out carry their code.
- Pins: himalaya 6f7c7a1f, pimalaya-cli 51ce8b1, pimalaya-config d5a33cb (`default` a name like any other). The re-resolve downgraded windows-sys in six lock entries; those lines were put back.

## Capabilities moved

- [configuration](../spec/configuration.md): *The account to open is named by `-a`* names the codes and `-a default`.
