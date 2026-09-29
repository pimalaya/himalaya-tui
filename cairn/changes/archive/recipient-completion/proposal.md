---
cairn: change
id: recipient-completion
status: landed
created: 2026-09-29
---

# Complete recipients from a user-supplied contact command

Issue #16 asks for contacts in the composer. Contacts are not Himalaya's business, so the TUI does not store, sync or parse address books: it asks a command, the aerc `address-book-cmd` way, so khard, abook, notmuch-address, cardamum or a hand-written script all plug in the same.

## What changes

`contact-command` is read at the global and the account level, the account winning. It takes both `CommandConfig` shapes. A `%s` in it is replaced by the typed fragment, shell-quoted for a shell line and verbatim inside an argv element; with no `%s`, the fragment is appended as a last argument, quoted the same way. Its stdout is read as the aerc format: one contact per line, the address then an optional tab and name, further tab-separated fields ignored.

`contact-complete-key` names the key triggering completion, `tab` by default, with optional `ctrl-`, `alt-` and `shift-` prefixes. It only acts in insert mode and only in the address part of a `To`, `Cc`, `Bcc` or `Reply-To` header, continuation lines included, before the blank line ending the headers. Anywhere else the key goes to the editor untouched, so Tab still inserts a tab in the body. `From` is left out: it names the sender's own identity, which an address book does not hold.

The fragment is what follows the last comma, or the colon, up to the cursor. The command runs on a thread and the event loop polls it, so a slow lookup never freezes the screen; it is killed after five seconds. A result is dropped if the buffer moved on meanwhile. No match or a failure reports on the status bar, one match is inserted directly, several open a list under the cursor (above it when the pane has no room below), navigated with the arrows, `Ctrl-n`/`Ctrl-p` or the complete key, accepted with `Enter`, dismissed with `Esc`; any other key dismisses it and reaches the editor.

The pick is inserted as `Name <address>`, the name quoted per RFC 5322 section 3.2.4 when it carries a special, or as the bare address when there is no name. The replacement goes through edtui's own delete and insert actions, so undo sees it.
