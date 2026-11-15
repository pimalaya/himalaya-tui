---
cairn: spec
capability: command-line
---

# Command line

How the binary reads its command line before any screen is drawn: the grammar and its refusals, and the standard commands, all as the himalaya CLI and the family's other binaries answer them.

### Requirement: A refused command line answers as himalaya's

The command line SHALL be parsed through pimalaya-cli's `parse`. Help and version SHALL print as clap prints them. A command line the grammar refuses SHALL exit 2 without drawing the interface: with a `--json` word before any `--`, the failure object coded `usage-invalid` on stdout and nothing on stderr, otherwise clap's rendering on stderr and nothing on stdout.

#### Scenario: An unknown flag under --json

Given `himalaya-tui --json --nope`, when it runs, then it exits 2 and prints on stdout the failure object with `code` `usage-invalid`, an `error` starting `unexpected argument '--nope'`, empty `sources` and a null `backtrace`.

#### Scenario: An unknown flag

Given `himalaya-tui --nope`, when it runs, then it exits 2 with clap's error on stderr and nothing on stdout.

#### Scenario: A --json word after the separator

Given `himalaya-tui --nope -- --json`, when it runs, then the refusal is clap's rendering on stderr, the word being no flag.

### Requirement: The standard commands are himalaya's

`completion` and `manual` SHALL be pimalaya-cli's commands, in himalaya's order, with the abouts pimalaya-cli gives them and `completions` and `manuals` as hidden aliases. A failure of either SHALL be reported as the start of the interface is, through himalaya's `ErrorCode`: the failure object on stdout, exit 1.

#### Scenario: manual with no argument

Given `himalaya-tui manual`, when it runs, then it exits 1 with `Error: Writing several manual pages to the standard output is ambiguous, use --dir to generate them as files` on stdout, as `himalaya manual` does, and under `--json` the failure object with a null `code`.

#### Scenario: The help

Given `himalaya-tui --help`, when it runs, then its commands are `completion` ("Generate completion script for the given shell(s)"), `manual` ("Generate manual page for the given command(s)") and `help`, in that order.
