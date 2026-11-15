---
cairn: change
id: the-family-command-line
status: landed
created: 2026-11-15
---

# The family's command line

Task T1 of the family's CLI alignment round of 2026-11-15 (finding F47 of its audit): himalaya-tui parsed its command line with clap's own `Cli::parse()` (src/main.rs:74), so a refused command line ignored `--json` and printed clap's rendering on stderr, where himalaya and every sibling print the failure object coded `usage-invalid` on stdout. It had no tests directory either.

## What changes

- The command line is parsed through pimalaya-cli's `pimalaya_cli::clap::parse`, as himalaya's src/main.rs does: exit 2 for a refused command line, clap's rendering on stderr, or under `--json` the `usage-invalid` failure object on stdout.
- `completion` and `manual` report a failure through `ErrorReport::<ErrorCode>::eval_coded` with himalaya's `error::coded`, as the start of the interface already did, so a failure carrying himalaya's code keeps it.
- `completion` and `manual` lose their own abouts ("Generate shell completion scripts", "Generate man pages") for the ones pimalaya-cli gives them, which himalaya prints; they already came in himalaya's order.
- pimalaya-cli 0.5.2, the requirement raised to `0.5.2`.
- tests/usage.rs drives the built binary: a refused command line (with and without `--json`), `completion` and `manual` with no argument (checked against `himalaya` run in an empty home: exit 1, the error on stdout, `code` null), the help listing, and a missing configuration (exit 1 naming the path, as text and under `--json`). No test opens the interface, which needs a terminal.

## Not covered

- `json-schema`: the TUI carries none, where every sibling does. Its own outputs are the standard commands' and the failure object; whether it takes the command is left to the owner.
- The pins on himalaya and mml stay as they are (T2).
