use std::collections::HashSet;

use io_imap::types::{
    envelope::Address,
    fetch::{MessageDataItem, MessageDataItemName},
    flag::FlagFetch,
};
use log::debug;
use rfc2047_decoder::{Decoder, RecoverStrategy};

use crate::app::Envelope;

pub(crate) fn parse_envelope(
    uid: u32,
    items: impl IntoIterator<Item = MessageDataItem<'static>>,
) -> Envelope {
    let mut envelope_uid = uid;
    let mut date = String::new();
    let mut from = String::new();
    let mut subject = String::new();
    let mut flags = HashSet::new();

    for item in items {
        match item {
            MessageDataItem::Uid(u) => {
                envelope_uid = u.get();
            }
            MessageDataItem::Envelope(env) => {
                if let Some(d) = &env.date.0 {
                    date = parse_date(&String::from_utf8_lossy(d.as_ref()));
                }
                if let Some(s) = &env.subject.0 {
                    subject = decode_mime(&String::from_utf8_lossy(s.as_ref()));
                }
                from = format_addresses_short(&env.from);
            }
            MessageDataItem::Flags(f) => {
                flags = f.into_iter().map(format_flag).collect();
            }
            _ => {}
        }
    }

    Envelope {
        id: envelope_uid.to_string(),
        date,
        from,
        subject,
        flags,
    }
}

pub(crate) fn format_flag(flag: FlagFetch<'_>) -> String {
    use io_imap::types::flag::Flag;

    match flag {
        FlagFetch::Flag(f) => match f {
            Flag::Answered => "\\Answered".to_string(),
            Flag::Flagged => "\\Flagged".to_string(),
            Flag::Deleted => "\\Deleted".to_string(),
            Flag::Seen => "\\Seen".to_string(),
            Flag::Draft => "\\Draft".to_string(),
            Flag::Keyword(kw) => String::from_utf8_lossy(kw.inner().as_ref()).to_string(),
            Flag::Extension(_) => "\\Extension".to_string(),
        },
        FlagFetch::Recent => "\\Recent".to_string(),
    }
}

pub(crate) fn decode_mime(s: &str) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    match decoder.decode(s.as_bytes()) {
        Ok(decoded) => decoded,
        Err(err) => {
            debug!("cannot decode rfc2047 string `{s}`: {err}");
            s.to_string()
        }
    }
}

pub(crate) fn parse_date(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 4 {
        format!("{} {}", parts[1], parts[2])
    } else {
        date_str.chars().take(12).collect()
    }
}

pub(crate) fn format_addresses_short(addrs: &[Address<'_>]) -> String {
    addrs
        .iter()
        .map(|addr| {
            if let Some(n) = &addr.name.0 {
                let name = decode_mime(&String::from_utf8_lossy(n.as_ref()));
                if !name.is_empty() {
                    return name;
                }
            }
            let mailbox = addr
                .mailbox
                .0
                .as_ref()
                .map(|m| String::from_utf8_lossy(m.as_ref()).to_string())
                .unwrap_or_default();
            let host = addr
                .host
                .0
                .as_ref()
                .map(|h| String::from_utf8_lossy(h.as_ref()).to_string())
                .unwrap_or_default();

            if !mailbox.is_empty() && !host.is_empty() {
                format!("{mailbox}@{host}")
            } else {
                mailbox
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn fetch_item_names_body_peek() -> Vec<MessageDataItemName<'static>> {
    vec![MessageDataItemName::BodyExt {
        section: None,
        partial: None,
        peek: true,
    }]
}
