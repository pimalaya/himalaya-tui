---
cairn: delta
change: account-flag-frees-the-seed
---

## ADDED Requirements

### Requirement: The account to open is named by `-a`, never positionally

`-a` addresses the configuration file and nothing else. The positional argument addresses the wizard and nothing else, so neither can stand for the other and the two are mutually exclusive. It is spelled `EMAIL`, an address being what it is reached for, even though the wizard accepts a server URL and a folder path there too.

#### Scenario: No account is named

Given neither `-a` nor a positional argument, when the TUI starts, then it opens the account flagged `default = true`.

#### Scenario: A name the file does not carry

Given `-a` naming an account absent from the file, when the TUI starts, then it fails listing the accounts the file does hold, and no wizard runs.

#### Scenario: A name and no file at all

Given `-a` and no configuration file at the resolved path, when the TUI starts, then it fails naming the path, and no wizard runs.

#### Scenario: Both are given

Given `-a` and a positional argument together, when the TUI starts, then it fails on the conflict before anything is loaded.

## MODIFIED Requirements

### Requirement: A run with no account to open falls back to an in-memory one

A session needs an account to open. When the configuration resolves none, a wizard builds one that lives for that session alone, proposing nothing for the configuration file.

#### Scenario: The wizard is seeded

Given a positional argument, when the TUI starts, then the account lookup is skipped, the wizard discovers from that value without prompting for it, and no warning is raised. The rest of the file still applies.

#### Scenario: The file is refused outright

Given `--no-config`, when the TUI starts, then the file is not read at all, the wizard prompts for its input, and no warning is raised.

#### Scenario: No configuration file

Given no file at the default paths nor at the one `-c` names, and no seed, when the TUI starts, then it warns naming the path it looked for and runs the wizard.

#### Scenario: A file carrying no default account

Given a configuration file whose accounts none flags `default`, when the TUI starts with no account named and no seed, then it warns naming what is missing and runs the wizard.

#### Scenario: The account the wizard produced

Given an account the wizard built, when the session ends, then nothing was written to disk and the configuration file is unchanged.
