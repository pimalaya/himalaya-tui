---
cairn: tasks
change: wizard-is-a-fallback
---

# Tasks

- [x] Rewrite the wizard's module header to lead with its purpose and name its triggers.
- [x] Carry the reason for falling back through the model build, and print it with `Spinner::failure` so it reaches stderr rather than the log file.
- [x] Leave `--no-config` silent, the wizard having been asked for.
- [x] Correct the same framing in src/main.rs, the README, config.sample.toml and the changelog.
