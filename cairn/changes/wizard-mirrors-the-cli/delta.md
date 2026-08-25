---
cairn: delta
change: wizard-mirrors-the-cli
---

## MODIFIED Requirements

### Requirement: A run with no account to open falls back to the CLI's wizard

A session needs an account to open. When the configuration resolves none, the wizard builds one, and it is the himalaya CLI's wizard prompt for prompt: one prompt for an email address, a server URL or a folder path, a parallel search of the services reachable from it, a pick list of those that answered, the authentication method the chosen service advertised, and a connection test before the account is accepted.

Two things SHALL differ from the CLI, and nothing else. `--no-config` forces the wizard on a run whose file resolves an account. And declining the offer to file the account prints nothing, where the CLI prints the document to stdout.

The wizard SHALL offer to write the account it built to the configuration file, creating the file or appending to the one already there, and SHALL open that account whichever way the offer was answered.

#### Scenario: The wizard is seeded

Given a positional argument, when the TUI starts, then the account lookup is skipped, the wizard takes that value as the answer to its first prompt, and no welcome is printed. The rest of the file still applies.

#### Scenario: The file is refused outright

Given `--no-config`, when the TUI starts, then the file is not read at all, the wizard prompts for its input, and no welcome is printed.

#### Scenario: No configuration file

Given no file at the default paths nor at the one `-c` names, and no seed, when the TUI starts, then it welcomes, names the path it looked for, and offers to generate an account; declining leaves the run with nothing to open and it stops there.

#### Scenario: A file carrying no default account

Given a configuration file whose accounts none flags `default`, when the TUI starts with no account named and no seed, then it warns naming what is missing and goes straight to the prompts, no welcome being owed for a file that is there.

#### Scenario: The offer is accepted

Given an account the wizard built and a configuration file already on disk, when the offer is accepted, then the `[accounts.<name>]` block is appended to that file with its comments and formatting untouched, under a name no other account takes, claiming the default only when no other account does.

#### Scenario: The offer is declined

Given an account the wizard built, when the offer is declined, then nothing is written, nothing is printed, and the session opens on that account all the same.

#### Scenario: There is no terminal to prompt on

Given a run whose standard input is not a terminal, when the wizard is reached, then it stops naming the documented sample rather than raising a prompt nobody can answer.

### Requirement: A configuration file valid for the CLI loads in the TUI

Blocks both binaries model (`imap`, `smtp`, `jmap`, `maildir`, `m2dir`, and the `tls` and `sasl` tables inside them) accept every option the CLI accepts there. Options the TUI has no use for are accepted and ignored rather than rejected. Blocks and fields only one binary models are tolerated by the other.

An account's identity SHALL be read under either spelling, `from` and `from-name` or `email` and `display-name`, and SHALL be written under the CLI's, the wizard filing into the file the CLI authors.

#### Scenario: An option the TUI does not act on

Given a configuration file setting `imap.sort.fallback`, which the CLI reads and the TUI does not, when the TUI loads that file, then the file loads and the option has no effect.

#### Scenario: A backend the TUI does not support

Given an account whose block configures `gmail`, when the TUI loads that file, then the block is ignored and the account's other backends stay usable.

#### Scenario: An identity written by the CLI

Given an account declaring `email` and `display-name`, when the TUI opens it, then the composer sends from that address under that name, exactly as if `from` and `from-name` had been used.

#### Scenario: An account the wizard filed

Given an account the wizard wrote, when the himalaya CLI loads that file, then the block reads as one of its own: the identity under `email` and `display-name`, the mailboxes under `mailbox.alias.*`, and no key it does not model.
