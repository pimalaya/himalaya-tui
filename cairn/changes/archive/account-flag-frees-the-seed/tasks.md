---
cairn: tasks
change: account-flag-frees-the-seed
---

# Tasks

- [x] Add `-a`/`--account` for the configured account name, and make the positional argument the wizard's seed alone.
- [x] Refuse the two together at the clap level.
- [x] Make a seed skip the account lookup outright, keeping the file for its globals, and warn about nothing.
- [x] Fail rather than fall back when `-a` names an account the file does not carry, or when there is no file at all.
- [x] Override `take_account` so the failure lists the accounts the file does hold.
- [x] Spell the positional `EMAIL`, and drop the same enumeration from the wizard's own prompt.
- [x] Show the seed rather than `unspecified` in the header when the wizard was seeded.
- [x] Follow through in the README, config.sample.toml and the changelog.
