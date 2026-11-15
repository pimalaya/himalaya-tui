---
cairn: log
change: the-family-command-line
landed: 2026-11-15
---

# The family's command line

Task T1 of the CLI alignment round of 2026-11-15 (finding F47). Capability **command-line** added, **configuration** moved.

## What landed

- src/main.rs: the command line is parsed by `pimalaya_cli::clap::parse::<Cli>()` where it was clap's `Cli::parse()`, so a refused line under `--json` prints the `usage-invalid` failure object on stdout and exits 2; a `completion` or `manual` failure goes through `himalaya::error::coded` and `ErrorReport::<ErrorCode>::eval_coded`, where it was uncoded `ErrorReport::eval`.
- src/cli.rs: the two standard commands take pimalaya-cli's abouts, as himalaya's; their order was already himalaya's.
- pimalaya-cli 0.5.0 to 0.5.2 in Cargo.lock, the requirement raised to `0.5.2`; `cargo update -p pimalaya-cli` also moved nine dependents off windows-sys 0.59 and dropped its entry, put back.
- tests/usage.rs, the first tests directory: eight tests through the built binary, run in a temp home with `TMPDIR` there for the log file. Three fail on the old sources (the `usage-invalid` object for `--json --nope` and `--json manual --nope`, the abouts); the other five pin what already matched himalaya: clap's rendering without `--json`, a `--json` after `--`, `completion` and `manual` with no argument (exit 1, the error on stdout, `code` null, the same answers as `himalaya` gives in an empty home), and a missing configuration (exit 1 naming the path, as text and as the failure object).

## Capabilities moved

- [command-line](../spec/command-line.md): new, *A refused command line answers as himalaya's* and *The standard commands are himalaya's*.
- [configuration](../spec/configuration.md): *No configuration stops before the screen* states exit 1 and the `--json` answer.
