mod stream;

use std::num::NonZeroU32;

use anyhow::{bail, Result};
use io_imap::{
    coroutines::{append::*, fetch::*, lsub::*, select::*},
    types::{
        core::{Literal, Vec1},
        extensions::binary::LiteralOrLiteral8,
        fetch::{MacroOrMessageDataItemNames, MessageDataItem, MessageDataItemName},
        flag::Flag,
        sequence::SequenceSet,
    },
};
use io_stream::runtimes::std::handle;
use log::debug;
use mail_parser::MessageParser;
use rfc2047_decoder::{Decoder, RecoverStrategy};

use crate::app::{Envelope, Mailbox};
use crate::config::ImapConfig;

pub use stream::{connect, Stream};

pub fn fetch_mailboxes(config: &ImapConfig) -> Result<Vec<Mailbox>> {
    let (context, mut stream) = connect(config.clone())?;

    let reference = "".try_into()?;
    let pattern = "*".try_into()?;

    let mut arg = None;
    let mut coroutine = ImapLsub::new(context, reference, pattern);

    let mailboxes = loop {
        match coroutine.resume(arg.take()) {
            ImapLsubResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapLsubResult::Ok { mailboxes, .. } => break mailboxes,
            ImapLsubResult::Err { err, .. } => bail!(err),
        }
    };

    let result = mailboxes
        .into_iter()
        .map(|(mbox, delim, _attrs)| {
            let name = match mbox {
                io_imap::types::mailbox::Mailbox::Inbox => "INBOX".to_string(),
                io_imap::types::mailbox::Mailbox::Other(mbox) => {
                    String::from_utf8_lossy(mbox.inner().as_ref()).to_string()
                }
            };
            let delimiter = delim.map(|d| d.inner());

            Mailbox {
                name,
                delimiter,
                subscribed: true,
            }
        })
        .collect();

    Ok(result)
}

pub fn fetch_envelopes(config: &ImapConfig, mailbox: &str) -> Result<Vec<Envelope>> {
    let (context, mut stream) = connect(config.clone())?;

    let mailbox_owned = mailbox.to_string();
    let mailbox_name = mailbox_owned.try_into()?;

    // SELECT mailbox
    let mut arg = None;
    let mut coroutine = ImapSelect::new(context, mailbox_name);

    let context = loop {
        match coroutine.resume(arg.take()) {
            ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapSelectResult::Ok { context, .. } => break context,
            ImapSelectResult::Err { err, .. } => bail!(err),
        }
    };

    // Parse sequence set for all messages
    let sequence_set: SequenceSet = "1:*".parse()?;

    // FETCH envelopes and flags
    let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
        MessageDataItemName::Envelope,
        MessageDataItemName::Flags,
    ]);

    let mut arg = None;
    let mut coroutine = ImapFetch::new(context, sequence_set, item_names, true);

    let data = loop {
        match coroutine.resume(arg.take()) {
            ImapFetchResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapFetchResult::Ok { data, .. } => break data,
            ImapFetchResult::Err { err, .. } => bail!(err),
        }
    };

    let mut envelopes: Vec<Envelope> = data
        .into_iter()
        .map(|(seq, items)| parse_envelope(seq, items))
        .collect();

    envelopes.sort_by(|a, b| b.uid.cmp(&a.uid));

    Ok(envelopes)
}

fn parse_envelope(seq: NonZeroU32, items: Vec1<MessageDataItem<'static>>) -> Envelope {
    let mut uid = seq.get();
    let mut date = String::new();
    let mut from = String::new();
    let mut subject = String::new();
    let mut flags = Vec::new();

    for item in items.into_iter() {
        match item {
            MessageDataItem::Uid(u) => {
                uid = u.get();
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
                flags = f.into_iter().map(|f| format_flag(f)).collect();
            }
            _ => {}
        }
    }

    Envelope {
        uid,
        date,
        from,
        subject,
        flags,
    }
}

fn format_flag(flag: io_imap::types::flag::FlagFetch<'_>) -> String {
    use io_imap::types::flag::{Flag, FlagFetch};

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

fn decode_mime(s: &str) -> String {
    let decoder = Decoder::new().too_long_encoded_word_strategy(RecoverStrategy::Decode);
    match decoder.decode(s.as_bytes()) {
        Ok(decoded) => decoded,
        Err(err) => {
            debug!("cannot decode rfc2047 string `{s}`: {err}");
            s.to_string()
        }
    }
}

fn parse_date(date_str: &str) -> String {
    // Simple date parsing - just extract day/month
    // Full format is typically: "Mon, 6 Jan 2025 10:30:00 +0000"
    let parts: Vec<&str> = date_str.split_whitespace().collect();
    if parts.len() >= 4 {
        format!("{} {}", parts[1], parts[2])
    } else {
        date_str.chars().take(12).collect()
    }
}

fn format_addresses_short(addrs: &[io_imap::types::envelope::Address<'_>]) -> String {
    addrs
        .iter()
        .map(|addr| {
            // If name exists, show decoded name only
            if let Some(n) = &addr.name.0 {
                let name = decode_mime(&String::from_utf8_lossy(n.as_ref()));
                if !name.is_empty() {
                    return name;
                }
            }
            // Otherwise show email
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

pub fn fetch_raw_message(config: &ImapConfig, mailbox: &str, uid: u32) -> Result<Vec<u8>> {
    let (context, mut stream) = connect(config.clone())?;

    let mailbox_owned = mailbox.to_string();
    let mailbox_name = mailbox_owned.try_into()?;

    let mut arg = None;
    let mut coroutine = ImapSelect::new(context, mailbox_name);

    let context = loop {
        match coroutine.resume(arg.take()) {
            ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapSelectResult::Ok { context, .. } => break context,
            ImapSelectResult::Err { err, .. } => bail!(err),
        }
    };

    let id = NonZeroU32::new(uid).ok_or_else(|| anyhow::anyhow!("UID must be non-zero"))?;

    let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
        MessageDataItemName::BodyExt {
            section: None,
            partial: None,
            peek: true,
        },
    ]);

    let mut arg = None;
    let mut coroutine = ImapFetchFirst::new(context, id, item_names, true);

    let items = loop {
        match coroutine.resume(arg.take()) {
            ImapFetchFirstResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapFetchFirstResult::Ok { items, .. } => break items,
            ImapFetchFirstResult::Err { err, .. } => bail!(err),
        }
    };

    let mut raw_message: Option<Vec<u8>> = None;
    for item in items.into_iter() {
        if let MessageDataItem::BodyExt { data, .. } = item {
            if let Some(data) = data.0 {
                raw_message = Some(data.as_ref().to_vec());
            }
        }
    }

    raw_message.ok_or_else(|| anyhow::anyhow!("No message data returned"))
}

pub fn fetch_message(config: &ImapConfig, mailbox: &str, uid: u32) -> Result<String> {
    let (context, mut stream) = connect(config.clone())?;

    let mailbox_owned = mailbox.to_string();
    let mailbox_name = mailbox_owned.try_into()?;

    // SELECT mailbox
    let mut arg = None;
    let mut coroutine = ImapSelect::new(context, mailbox_name);

    let context = loop {
        match coroutine.resume(arg.take()) {
            ImapSelectResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapSelectResult::Ok { context, .. } => break context,
            ImapSelectResult::Err { err, .. } => bail!(err),
        }
    };

    // FETCH with BODY.PEEK[] to avoid marking as read
    let id = NonZeroU32::new(uid).ok_or_else(|| anyhow::anyhow!("UID must be non-zero"))?;

    let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
        MessageDataItemName::BodyExt {
            section: None,
            partial: None,
            peek: true,
        },
    ]);

    let mut arg = None;
    let mut coroutine = ImapFetchFirst::new(context, id, item_names, true);

    let items = loop {
        match coroutine.resume(arg.take()) {
            ImapFetchFirstResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapFetchFirstResult::Ok { items, .. } => break items,
            ImapFetchFirstResult::Err { err, .. } => bail!(err),
        }
    };

    // Extract raw message bytes
    let mut raw_message: Option<Vec<u8>> = None;
    for item in items.into_iter() {
        if let MessageDataItem::BodyExt { data, .. } = item {
            if let Some(data) = data.0 {
                raw_message = Some(data.as_ref().to_vec());
            }
        }
    }

    let raw = raw_message.ok_or_else(|| anyhow::anyhow!("No message data returned"))?;

    // Parse message using mail-parser and get text content
    let message = MessageParser::default()
        .parse(&raw)
        .ok_or_else(|| anyhow::anyhow!("Failed to parse message"))?;

    // Get plain text, or convert HTML to text
    let content = if let Some(text) = message.body_text(0) {
        text.to_string()
    } else if let Some(html) = message.body_html(0) {
        html2text::from_read(html.as_bytes(), 80)
    } else {
        // Fallback to raw message as string
        String::from_utf8_lossy(&raw).to_string()
    };

    Ok(content)
}

pub fn save_to_drafts(config: &ImapConfig, content: &str) -> Result<()> {
    let (context, mut stream) = connect(config.clone())?;

    // Build a minimal RFC 5322 message
    let message_content = format!(
        "From: \r\nTo: \r\nSubject: Draft\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}",
        content
    );

    let mailbox: io_imap::types::mailbox::Mailbox<'static> = "Drafts".try_into()?;
    let literal = Literal::try_from(message_content.into_bytes())?;
    let message = LiteralOrLiteral8::Literal(literal);

    // Add Draft flag
    let flags = vec![Flag::Draft];

    // APPEND
    let mut arg = None;
    let mut coroutine = ImapAppend::new(context, mailbox, flags, None, message);

    loop {
        match coroutine.resume(arg.take()) {
            ImapAppendResult::Io { io } => arg = Some(handle(&mut stream, io)?),
            ImapAppendResult::Ok { .. } => break,
            ImapAppendResult::Err { err, .. } => bail!(err),
        }
    }

    Ok(())
}
