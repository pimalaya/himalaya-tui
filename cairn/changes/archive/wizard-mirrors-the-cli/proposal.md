---
cairn: change
id: wizard-mirrors-the-cli
status: landed
created: 2026-08-25
---

# Run the himalaya CLI's wizard, and let it file what it finds

The TUI grew a wizard of its own because it had to open something when the configuration resolved no account, and it was shaped by that: probe PACC, then autoconfig, then SRV, first hit wins, ask straight for a SASL mechanism from a fixed list of six, keep the result in memory. The CLI's wizard, meanwhile, searches every mechanism in parallel, offers what actually answered as a pick list, narrows the IMAP mechanisms by a live CAPABILITY probe, tests each connection as it configures it, reads special-use mailboxes off the session, and stores secrets through the OS keyring and OAuth broker pickers instead of raw in the file.

Two wizards over one configuration file is one too many, and the weaker one is here. Someone who set an account up with the CLI and then opens the TUI meets a different flow asking different questions, and the account the TUI builds could not have been written by the CLI even if it wanted to write it.

## What changes

The CLI's wizard is the wizard: src/wizard/ becomes search, discover, imap_smtp, jmap, local, mailbox, secret and configure, the CLI's modules adapted to this crate's config types, and the serial pacc/autoconfig/srv probes are deleted with the module that chained them. io-pim-discovery gains the `rfc8620` feature the parallel search needs. Gmail and Microsoft Graph are discovered and then dropped, this binary having no such backend, the same way a CLI build without those features drops them.

Two things differ, and only two. `--no-config` forces the wizard on a run whose file resolves an account perfectly well, which the CLI has no use for. And declining the offer to save prints nothing, where the CLI prints the document to stdout: a configuration file is all the CLI has to give, whereas here the account is opened either way. The offer itself is kept rather than dropped, so that stdout is the whole of the difference.

Filing an account means writing one, which the TUI has never done. `AccountConfig` gains the CLI's `render`, its key order and its `skip_serializing_if` attributes so the block comes out identical, plus the `mailbox.alias.*` table the CLI addresses mailboxes by. The identity is written under the CLI's spelling, `email` and `display-name`, since the file it lands in is the one the CLI authors.

`footer!` closes `--help`, as it does on every other Pimalaya binary.
