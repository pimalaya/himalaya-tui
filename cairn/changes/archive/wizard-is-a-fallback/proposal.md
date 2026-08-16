---
cairn: change
id: wizard-is-a-fallback
status: landed
created: 2026-08-16
---

# Say what the wizard is for, and warn when it takes over

The wizard's module header framed itself against the himalaya CLI's, reducing the difference to serial versus parallel probing and claiming the prompts were kept identical on purpose. That reads the two as variants of one tool, and invites the conclusion that this one is a generation behind. They do different jobs. The CLI's wizard authors a configuration, proposing an `[accounts.<name>]` table that outlives the run. The TUI writes no configuration at all, reading the file the CLI already wrote, so its wizard exists only to give a run with nothing to open something to open, for that session alone.

Serial probing with no picker follows from that purpose rather than trailing it: a throwaway account wants the fewest questions that produce a usable session.

The behaviour worth changing is the silence around it. Falling back was indistinguishable from configuring on purpose, so a mistyped `-c` path or a forgotten `default = true` dropped the user into prompts with no hint of why. The two accidental triggers should say what was missing before the prompts start; `--no-config` should not, having asked for the wizard.
