---
cairn: log
change: recipient-completion
landed: 2026-09-29
---

# Recipients complete from a contact command

Issue #16. src/contact.rs holds the whole mechanism: `ContactTarget::locate` finds the fragment under the cursor in a `To`, `Cc`, `Bcc` or `Reply-To` header or its continuation lines, `ContactLookup` runs `contact-command` on a thread with a 5 second kill, `Contact` parses the aerc output and renders an RFC 5322 mailbox, and `ContactCompleteKey` parses `contact-complete-key`.

A shell line gets the fragment single-quoted, and a `'%s'` or `"%s"` the user already quoted is unwrapped first, so aerc lines port unchanged. An argv gets it verbatim. Without a placeholder it is appended. Exit 1 with empty stdout is no match (khard, abook), any other failure reaches the status bar with the first stderr line.

The event loop polls the lookup every iteration, dropping its wait from 250 to 25 ms while one runs. An answer is discarded when the fragment at the cursor no longer matches the queried one. Several matches open a list rendered after the editor, whose render is what measures the cursor, below it or above it when the pane has no room. The pick replaces the fragment through edtui's `DeleteChar` and `InsertChar`, so undo sees it. `From` does not complete: it is the sender's identity, not a contact.

Unrelated to the feature but needed to build: io-smtp 0.4 added `SmtpMessageSendOptions` to `send`, and src/smtp/backend.rs passes its default, which strips `Bcc` from the transmitted headers.

New capability: cairn/spec/contacts.md.
