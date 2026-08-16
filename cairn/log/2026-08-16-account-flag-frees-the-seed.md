---
cairn: log
change: account-flag-frees-the-seed
landed: 2026-08-16
---

# Split the overloaded positional into `-a` and a wizard seed

One positional argument carried two jobs that pull in opposite directions. Named `ACCOUNT-OR-SERVER`, it was passed both to the account lookup and to the wizard, so `himalaya-tui fastmail.com` meant "open the account called fastmail.com" and "discover an account at fastmail.com" at once. Which one you got depended on whether a configuration file happened to exist, and with one on disk the discovery half was unreachable: pimalaya-config bails on an unknown named account, so the run died with `Get account fastmail.com error` rather than discovering anything.

The two jobs now have their own argument. `-a`, matching the himalaya CLI's flag, addresses the configuration file: it opens the account it names, defaults to the one flagged `default = true`, and errors on a name the file does not carry. The positional argument, now `EMAIL`, addresses the wizard alone: it skips the account lookup and opens a throwaway account discovered from that value. Clap refuses the two together, so nothing has to guess which was meant.

It is spelled `EMAIL` rather than naming the three shapes it accepts, following the rename the CLI's wizard made for the same reason: listing every accepted shape up front reads as a question about which one to pick, where an address is what almost everyone types. A server URL and a folder path still work, and the wizard's own prompt lost the same enumeration, becoming plain `Email:` like the CLI's, with the broader vocabulary kept for the error raised on empty input.

Passing a seed keeps the configuration file for its globals, the theme, the signature and the keybindings, and only swaps the account out, which is what distinguishes it from `--no-config` where the file is dropped whole. Both ask for the wizard, so neither warns; the accidental triggers, a missing file and a missing default account, still do.

An unknown account name used to fail with pimalaya-config's `Get account <name> error`, which left the user guessing. Now that `-a` is the only way to reach that path, `take_account` is overridden on the TUI's `Config` to list the accounts the file does hold, or to say the file declares none at all, satisfying the CLI guideline on how a missing named account reports itself. A `-a` pointed at a path holding no file at all fails the same way, naming the path: the flag addresses the file and nothing else, so falling back would open something other than what was asked for.

Verified by running the binary against a two-account file: the default account and `-a two` each connect to their own port, an unknown `-a` lists both names without touching the wizard, a seed goes straight to discovery with no warning, and `-a` with a seed is refused by clap.

The capability file cairn/spec/configuration.md gained a requirement covering the split and its three scenarios, and the fallback requirement gained the two ask-for-it scenarios next to the two accidental ones.
