---
cairn: delta
change: the-family-command-line
---

## ADDED Requirements

### Requirement: A refused command line answers as himalaya's

(command-line, a new capability) The command line SHALL be parsed through pimalaya-cli's `parse`. Help and version SHALL print as clap prints them. A command line the grammar refuses SHALL exit 2 without drawing the interface: with a `--json` word before any `--`, the failure object coded `usage-invalid` on stdout and nothing on stderr, otherwise clap's rendering on stderr and nothing on stdout.

#### Scenario: An unknown flag under --json

Given `himalaya-tui --json --nope`, when it runs, then it exits 2 and prints on stdout the failure object with `code` `usage-invalid`, an `error` starting `unexpected argument '--nope'`, empty `sources` and a null `backtrace`.

#### Scenario: An unknown flag

Given `himalaya-tui --nope`, when it runs, then it exits 2 with clap's error on stderr and nothing on stdout.

#### Scenario: A --json word after the separator

Given `himalaya-tui --nope -- --json`, when it runs, then the refusal is clap's rendering on stderr, the word being no flag.

### Requirement: The standard commands are himalaya's

(command-line) `completion` and `manual` SHALL be pimalaya-cli's commands, in himalaya's order, with the abouts pimalaya-cli gives them and `completions` and `manuals` as hidden aliases. A failure of either SHALL be reported as the start of the interface is, through himalaya's `ErrorCode`: the failure object on stdout, exit 1.

#### Scenario: manual with no argument

Given `himalaya-tui manual`, when it runs, then it exits 1 with `Error: Writing several manual pages to the standard output is ambiguous, use --dir to generate them as files` on stdout, as `himalaya manual` does, and under `--json` the failure object with a null `code`.

#### Scenario: The help

Given `himalaya-tui --help`, when it runs, then its commands are `completion` ("Generate completion script for the given shell(s)"), `manual` ("Generate manual page for the given command(s)") and `help`, in that order.

## MODIFIED Requirements

### Requirement: No configuration stops before the screen

(configuration) With no configuration file, or none resolving the account to open, the TUI SHALL fail before taking the screen with himalaya's error naming the path looked at and the documented sample, and exit 1. Under `--json` the error SHALL be the failure object on stdout.

#### Scenario: No file at all

Given no configuration file, when the TUI starts, then it prints the path it looked at and the sample's location, and exits 1 without drawing the interface.

#### Scenario: No file, under --json

Given no configuration file and no `-c`, when `himalaya-tui --json` starts, then it prints the failure object naming `<XDG_CONFIG_HOME>/himalaya/config.toml` and the sample on stdout, nothing on stderr, and exits 1.
