---
cairn: log
change: wizard-mirrors-the-cli
landed: 2026-08-25
---

# The wizard is the himalaya CLI's, and it files what it finds

src/wizard/ is now the CLI's wizard adapted to this crate's config types: search runs io-pim-discovery's parallel `compose_all_within` over IMAP, SMTP and JMAP and folds each service's advertised auth into an `AuthCaps`; discover orients on the input shape, offers what answered as a pick list and dispatches; imap_smtp narrows the SASL mechanisms by a live CAPABILITY probe, tests IMAP, then asks whether SMTP reuses the credential and tests it too; jmap picks the HTTP scheme among the advertised ones and reads the role-based mailbox aliases off the session that validated it; local detects a Maildir or an m2dir from the markers in the folder; secret routes every credential through the OS keyring and OAuth broker pickers. The serial pacc, autoconfig and srv probes are gone with the module that chained them, io-pim-discovery gained `rfc8620`, and check holds the connection tests, each of them the ordinary client constructor.

Gmail and Microsoft Graph are discovered and then dropped by `retain_supported`, this binary having no such backend, exactly as a CLI build without those features drops them.

configure is the CLI's `configure` command with the printing taken out: it welcomes when no file was found, derives the account name, claims the default only when no other account does, and offers to write or append the block. Declining prints nothing and the account is opened anyway, which is the only difference besides `--no-config` forcing the wizard in the first place. Writing an account at all is new here, so `AccountConfig` gained the CLI's `render` with its key order, `skip_serializing_if` on the defaulted scalars so a generated block holds only what was configured, a `MailboxConfig` for the `mailbox.alias.*` table the CLI addresses mailboxes by, and the identity serialized under `email` and `display-name`.

cli.rs carries `footer!` on `--help` and hands the wizard the path a missing file was looked for at; the account name now comes back from the wizard rather than from the seed or a placeholder. The welcome and its offer are skipped when standard input is not a terminal, the wizard failing on that condition with a message that names it.

The configuration capability moved: the fallback requirement is now the CLI's wizard with its two named differences and the save offer, and the shared-file requirement says the identity is written under the CLI's spelling. The header no longer claims the TUI writes nothing to disk.
