---
cairn: delta
change: wizard-is-a-fallback
---

## ADDED Requirements

### Requirement: A run with no account to open falls back to an in-memory one

A session needs an account to open. When the configuration resolves none, a wizard builds one that lives for that session alone, proposing nothing for the configuration file.

#### Scenario: No configuration file

Given no file at the default paths nor at the one `-c` names, when the TUI starts, then it warns naming the path it looked for and runs the wizard.

#### Scenario: A file carrying no default account

Given a configuration file whose accounts none flags `default`, when the TUI starts with no account named, then it warns naming what is missing and runs the wizard.

#### Scenario: The wizard was asked for

Given `--no-config`, when the TUI starts, then it runs the wizard against no file and raises no warning.

#### Scenario: The account the wizard produced

Given an account the wizard built, when the session ends, then nothing was written to disk and the configuration file is unchanged.
