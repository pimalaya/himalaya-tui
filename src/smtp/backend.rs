//! # SMTP backend
//!
//! SMTP adapter for the shared cross-protocol client: send-only, it is
//! the sending transport of storage backends that cannot send
//! themselves, IMAP and Maildir.
//!
//! The RFC 5321 envelope is derived from the raw message headers, From:
//! giving the reverse path and To:, Cc:, Bcc: the forward paths.

use std::borrow::Cow;

use anyhow::{Result, anyhow, bail};
use io_smtp::{
    client::SmtpClient as _,
    message::SmtpMessageSendOptions,
    rfc5321::{
        SmtpDomain, SmtpEhloDomain, SmtpForwardPath, SmtpLocalPart, SmtpMailbox, SmtpReversePath,
    },
};
use mime::{
    header::marker::{BCC, CC, FROM, TO},
    tree::{cst::MimeCst, parse::MimeParseOptions},
    value::address::MimeAddressList,
};

use crate::smtp::client::SmtpClient;

impl SmtpClient {
    /// Runs the RFC 5321 mail transaction: MAIL FROM, RCPT TO, DATA.
    ///
    /// The envelope is derived from the headers of `raw`.
    pub fn send_message(&mut self, raw: Vec<u8>) -> Result<()> {
        let (reverse, forwards) = envelope(&raw)?;
        let reverse = parse_smtp_mailbox(&reverse)?;
        let forwards = forwards
            .iter()
            .map(|address| parse_smtp_mailbox(address))
            .collect::<Result<Vec<_>>>()?;

        let reverse_path = SmtpReversePath::SmtpMailbox(reverse);
        let forward_paths: Vec<SmtpForwardPath<'static>> =
            forwards.into_iter().map(SmtpForwardPath::from).collect();

        self.send(
            reverse_path,
            forward_paths,
            raw,
            SmtpMessageSendOptions::default(),
        )?;
        Ok(())
    }
}

/// The bare reverse path and forward paths of `raw`, read off its header
/// block: the first `From:` address, then every `To:`, `Cc:` and `Bcc:`
/// one in header order, groups flattened into their members.
///
/// A message with no `From:` address or no recipient is refused, since
/// no transaction can carry it.
fn envelope(raw: &[u8]) -> Result<(String, Vec<String>)> {
    // NOTE: the header block alone, the body left unsplit.
    let opts = MimeParseOptions {
        structure_depth: Some(0),
        ..Default::default()
    };
    let parsed = MimeCst::parse_with(raw, opts);
    let root = &parsed.root;

    let reverse = root
        .header::<FROM>()
        .and_then(|list| addresses(&list).into_iter().next())
        .ok_or_else(|| anyhow!("No `From:` header found in raw message"))?;

    let forwards: Vec<String> = [
        root.header::<TO>(),
        root.header::<CC>(),
        root.header::<BCC>(),
    ]
    .into_iter()
    .flatten()
    .flat_map(|list| addresses(&list))
    .collect();

    if forwards.is_empty() {
        bail!("No `To:` / `Cc:` / `Bcc:` recipients found in raw message");
    }

    Ok((reverse, forwards))
}

/// Flattens an address list, groups included, into bare
/// `local-part@domain` strings.
fn addresses(list: &MimeAddressList) -> Vec<String> {
    list.mailboxes()
        .filter_map(|mailbox| mailbox.address.clone())
        .filter(|email| !email.is_empty())
        .collect()
}

/// Parses `local-part@domain` into an owned SMTP mailbox.
fn parse_smtp_mailbox(address: &str) -> Result<SmtpMailbox<'static>> {
    let (local, domain) = address
        .rsplit_once('@')
        .ok_or_else(|| anyhow!("Invalid email address `{address}` in envelope"))?;
    if local.is_empty() || domain.is_empty() {
        bail!("Invalid email address `{address}` in envelope");
    }

    Ok(SmtpMailbox {
        local_part: SmtpLocalPart(Cow::Owned(local.to_string())),
        domain: SmtpEhloDomain::SmtpDomain(SmtpDomain(Cow::Owned(domain.to_string()))),
    })
}

#[cfg(test)]
mod tests {
    use super::envelope;

    /// Every recipient header feeds the envelope in header order, a group
    /// giving its members and a display name its bare address.
    #[test]
    fn every_recipient_header_feeds_the_envelope() {
        let raw = concat!(
            "From: Alice <a@x.org>, Other <o@x.org>\r\n",
            "To: Team: b@y.org, \"Carol, C\" <c@y.org>;\r\n",
            "Cc: =?utf-8?q?D=C3=A9?= <d@y.org>\r\n",
            "Bcc: e@y.org\r\n",
            "\r\n",
            "body",
        );

        let (from, rcpts) = envelope(raw.as_bytes()).unwrap();

        assert_eq!(from, "a@x.org");
        assert_eq!(rcpts, ["b@y.org", "c@y.org", "d@y.org", "e@y.org"]);
    }

    /// A message no transport can submit is refused before connecting.
    #[test]
    fn no_sender_or_no_recipient_is_refused() {
        let no_from = envelope(b"To: b@y.org\r\n\r\nbody");
        let no_rcpt = envelope(b"From: a@x.org\r\n\r\nbody");

        assert!(no_from.unwrap_err().to_string().contains("From:"));
        assert!(no_rcpt.unwrap_err().to_string().contains("recipients"));
    }
}
