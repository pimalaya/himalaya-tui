---
cairn: log
change: wizard-is-a-fallback
landed: 2026-08-16
---

# Stated what the wizard is for, and warned when it takes over

The wizard's own module header opened by comparing itself to the himalaya CLI's, calling the difference a matter of serial versus parallel probing and claiming the prompts were "deliberately kept identical". That framing reads the two as variants of one tool and invites the conclusion that this one trails the other, which is wrong: they do different jobs. The CLI's wizard authors a configuration, proposing an `[accounts.<name>]` table that outlives the run. The TUI writes no configuration at all, since it reads the file the CLI already wrote, and its wizard exists only so that a run resolving no account still has something to open. What it produces lives for the session and is never proposed for the file.

Serial probing with no picker follows from that purpose rather than lagging behind it: a throwaway account wants the fewest questions that yield a usable session. The header now leads with the purpose and names the three triggers, and the same correction landed in src/main.rs, the README (a new "Starting without a configuration" section replacing a paragraph that read as setup instructions), config.sample.toml and the changelog.

The behaviour that did change is the silence. Falling back to the wizard was indistinguishable from configuring one on purpose, so a mistyped `-c` path or a forgotten `default = true` dropped the user into prompts with no hint of why. The two accidental triggers now warn through `Spinner::failure`, which stops the spinner and prints on stderr, naming the path that was looked for or the default account that is missing. `--no-config` stays silent, having asked for the wizard.

The capability file cairn/spec/configuration.md gained a requirement covering the fallback, its three triggers and the guarantee that nothing reaches disk, and its opening paragraph now states that the TUI only ever reads the file.

One contradiction was found and deliberately left alone, since resolving it is a decision rather than a repair: the positional argument is documented as an account name *or* a discovery seed, but a seed only reaches the wizard when no configuration file was found. With a file on disk, `himalaya-tui fastmail.com` fails with `Get account fastmail.com error` from pimalaya-config rather than discovering anything.
