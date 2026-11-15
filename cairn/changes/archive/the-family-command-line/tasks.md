---
cairn: tasks
change: the-family-command-line
---

# Tasks

- [x] Tests first in tests/usage.rs; the three on the changed behaviour (the `usage-invalid` object for a refused line and a refused subcommand line, the abouts) fail on the old src/main.rs and src/cli.rs, the five others pin what already matched himalaya.
- [x] src/main.rs: `pimalaya_cli::clap::parse::<Cli>()`; the subcommand's result through `error::coded` and `ErrorReport::<ErrorCode>::eval_coded`.
- [x] src/cli.rs: the variant doc comments of `completion` and `manual` removed, so pimalaya-cli's abouts show.
- [x] Cargo.toml: pimalaya-cli `0.5.2` in both sections; Cargo.lock moved by `cargo update -p pimalaya-cli`, its windows-sys re-resolve put back so the diff holds that crate alone.
- [x] Spec: command-line.md added, configuration.md's missing-configuration requirement states the exit status and the `--json` answer; log written; CHANGELOG.
