---
cairn: change
id: account-flag-frees-the-seed
status: landed
created: 2026-08-16
---

# Give the account name its own flag and free the positional for the wizard

One positional argument carried two jobs pulling in opposite directions. Named `ACCOUNT-OR-SERVER`, it fed both the account lookup and the wizard, so `himalaya-tui fastmail.com` meant "open the account called fastmail.com" and "discover an account at fastmail.com" at once. Which one the user got depended on whether a configuration file happened to exist, and with one on disk the discovery half was unreachable: pimalaya-config bails on an unknown named account, so the run died with `Get account fastmail.com error` rather than discovering anything.

Splitting them is what the himalaya CLI already does. `-a` addresses the configuration file: it opens the account it names and defaults to the one flagged `default = true`. The positional argument addresses the wizard alone. Neither can stand for the other, so the two are mutually exclusive and nothing has to guess.

A seed should force the wizard rather than be ignored when the configuration happens to resolve an account, otherwise the same footgun returns in reverse and a working configuration silently swallows the request. It differs from `--no-config` in what survives: the file still supplies the theme, the signature and the keybindings, and only the account is swapped out.

The positional is spelled `EMAIL` rather than naming every shape it accepts, following the rename the CLI's wizard made for the same reason: listing all three read as a question about which one to pick, where an address is what almost everyone types.

Now that `-a` is the only route to a named account, its failure has to be worth reading. pimalaya-config's `Get account <name> error` leaves the user guessing, where the CLI guidelines ask a missing named account to list the ones the configuration does hold.
