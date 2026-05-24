// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026  soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! MIME helpers shared by the read and send paths.
//!
//! [`decode_message_body`] picks a displayable body (text > html >
//! lossy-utf8) from a raw RFC 5322 byte string. [`extract_envelope`]
//! parses the headers to derive the SMTP MAIL FROM / RCPT TO list used
//! when sending a compiled message.

use std::collections::HashSet;

use anyhow::{Result, anyhow};
use mail_parser::{Addr, Address, HeaderName, HeaderValue, MessageParser};

pub fn decode_message_body(raw: &[u8]) -> Result<String> {
    let message = MessageParser::default()
        .parse(raw)
        .ok_or_else(|| anyhow!("Failed to parse message"))?;

    if let Some(text) = message.body_text(0) {
        Ok(text.to_string())
    } else if let Some(html) = message.body_html(0) {
        Ok(html2text::from_read(html.as_bytes(), 80)?)
    } else {
        Ok(String::from_utf8_lossy(raw).to_string())
    }
}

/// Extracts the envelope sender and recipients from the raw RFC 5322
/// message headers. Used for SMTP routing on send.
pub fn extract_envelope(raw: &[u8]) -> Result<(String, Vec<String>)> {
    let msg = MessageParser::new()
        .parse_headers(raw)
        .ok_or_else(|| anyhow!("Invalid message to send"))?;

    let mut mail_from: Option<String> = None;
    let mut rcpt_to: HashSet<String> = HashSet::new();

    for header in msg.headers() {
        let key = &header.name;
        let val = header.value();

        match key {
            HeaderName::From => {
                if let HeaderValue::Address(Address::List(addrs)) = val {
                    if let Some(email) = addrs.first().and_then(valid_email) {
                        mail_from = Some(email);
                    }
                } else if let HeaderValue::Address(Address::Group(groups)) = val {
                    if let Some(group) = groups.first() {
                        if let Some(email) = group.addresses.first().and_then(valid_email) {
                            mail_from = Some(email);
                        }
                    }
                }
            }
            HeaderName::To | HeaderName::Cc | HeaderName::Bcc => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    rcpt_to.extend(addrs.iter().filter_map(valid_email));
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    rcpt_to.extend(
                        groups
                            .iter()
                            .flat_map(|group| group.addresses.iter())
                            .filter_map(valid_email),
                    );
                }
                _ => (),
            },
            _ => (),
        };
    }

    let mail_from = mail_from.ok_or_else(|| anyhow!("The message does not contain any sender"))?;
    if rcpt_to.is_empty() {
        anyhow::bail!("The message does not contain any recipient");
    }

    Ok((mail_from, rcpt_to.into_iter().collect()))
}

fn valid_email(addr: &Addr) -> Option<String> {
    addr.address.as_ref().and_then(|email| {
        let email = email.trim();
        if email.is_empty() {
            None
        } else {
            Some(email.to_string())
        }
    })
}
