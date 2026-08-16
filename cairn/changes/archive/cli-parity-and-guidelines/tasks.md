---
cairn: tasks
change: cli-parity-and-guidelines
---

# Tasks

## Behaviour

- [x] Replace the three permissive server parsers with the CLI's `parse_server`, validating the scheme and reading a bare authority as one.
- [x] Accept `unix://` for IMAP and SMTP, and negotiate no SASL over it (PREAUTH).
- [x] Fail the connection when the host cannot be derived from the server URL, instead of authenticating against an empty host.
- [x] Retry an RFC 2822 date without its day-of-week token when chrono rejects the pair.
- [x] Open a fresh authenticated connection for a JMAP blob whose authority differs from the API's.
- [x] Resolve a JMAP mailbox name to its opaque id at the shared client, before dispatch.
- [x] Connect the SMTP transport on the first send rather than at startup.
- [x] Ask the system resolver for discovery, with an env override, before falling back to a public one.

## Alignment

- [x] Take the ALPN defaults from io-imap, io-smtp and io-jmap rather than restating them.
- [x] Move `Theme` next to the modules it aggregates, dropping the private module and its re-export.
- [x] Sort the manifest's dependencies and drop the unused ones.
- [x] Fix the preset names in the configuration docs and turn the dash lists in the module headers into prose.

## Documentation

- [x] Fence the README's shell blocks with `sh`, and point at the CLI's provider recipes.
- [x] Refresh the sample configuration: the ortie command line, the retired secret manager, the missing preset.
- [x] Refresh the contributing guide: the cairn reading step, the patch table, the SASL crate, the backticked paths.
- [x] List the missing preset in the changelog.
