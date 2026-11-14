---
cairn: delta
change: delete-moves-to-the-trash
---

## ADDED Requirements

### Requirement: Delete moves to the trash first

Delete, from the envelope dialog or the `Delete` key, SHALL run himalaya's `message delete` on the marked envelopes, else the focused one: the messages move to the account's trash, and only messages already in the trash are removed for good. In a mailbox whose role is `trash`, the TUI SHALL ask before deleting, and declining SHALL leave everything in place. The status bar SHALL say what happened from the receipt, and the page SHALL be listed again.

#### Scenario: From the inbox

Given a focused envelope in the inbox and a trash mailbox, when it is deleted, then nothing asks, the message moves to the trash, the status bar reads `Moved 1 message to the trash`, and the page no longer shows it.

#### Scenario: From the trash

Given two marked envelopes in the trash, when they are deleted, then the TUI asks `Delete 2 messages for good?`, and once confirmed they are removed and the status bar reads `Deleted 2 messages for good`.

#### Scenario: An IMAP server without UIDPLUS

Given a focused envelope in the trash of a server that does not support UIDPLUS, when it is deleted and confirmed, then it is flagged deleted and the status bar says the server expunges it later.

#### Scenario: No trash

Given an account whose backend reports no trash role and no `mailbox.alias.trash`, when an envelope is deleted, then the delete fails with himalaya's error naming `mailbox.alias.trash`, and the envelope stays.

## MODIFIED Requirements

## REMOVED Requirements
