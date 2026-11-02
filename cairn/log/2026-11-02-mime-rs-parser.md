---
cairn: log
change: mime-rs-parser
landed: 2026-11-02
---

# Parsed messages with mime-rs instead of mail-parser

The direct `mail-parser = "0.11"` dependency is gone, replaced by mime-rs pinned at `d7ea032`, the rev the himalaya CLI pins, and every reader follows the CLI's patterns.

- src/smtp/backend.rs derives the RFC 5321 envelope through `envelope`, which reads the header block alone (`structure_depth: Some(0)`): the first `From:` mailbox, then every `To:`, `Cc:` and `Bcc:` mailbox in header order, groups flattened, as `SubmissionEnvelope::parse` does in the CLI.
- src/imap/backend.rs reads the `Message-ID` it searches for, when the server sends no `APPENDUID`, through the `MESSAGE_ID` marker.
- src/maildir/backend.rs builds envelopes in `envelope_from_raw` from `MimeCst::parse` rather than io-maildir's `MaildirFullEntry::parsed`. `has_attachment` stays true for any part of the RFC 8621 attachment list, an inline image included, which is what mail-parser's `attachment_count` counted; the CLI's `has_attachment` leaves inline parts out, and IMAP here counts `attachment` dispositions only, so the three still disagree on an inline image.
- src/tui/update.rs `decode_message_body` shows the first part of the RFC 8621 text body, then of the HTML body, then the raw bytes. An HTML part goes through html2text's undecorated config at 80 columns. mail-parser rendered an HTML-only body with its own tag stripper, which ran headings and list items together; html2text keeps the layout, so such a body now reads differently. A two-word paragraph renders the same.

Reply and forward still parse with mail-parser: mml's `MmlTemplateReplyBuilder::build`, `MmlTemplateForwardBuilder::build` and `interpret_msg` take a `mail_parser::Message`, so the source goes through mml's own `MmlMimeLayer::parse` and mail-parser stays in the lockfile through mime-meta-language. It also stays through io-maildir's `parser` feature, which nothing calls any more.

mail-parser was built without `full_encoding`, so Shift_JIS, Big5, GBK and EUC-KR bodies and encoded words read as replacement characters. mime-rs decodes them through encoding_rs. Reply and forward quotes, still on mail-parser, do not gain this.

Characterisation tests for the SMTP envelope, the Maildir envelope and the body decoding were written against mail-parser first and pass unchanged on mime-rs, the HTML-only case excepted.

No capability moved: the spec states none of this.
