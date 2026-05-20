use anyhow::{anyhow, Result};
use io_jmap::{
    client::JmapClientStd,
    rfc8621::email::{Email, EmailAddress},
};

pub struct JmapMessageGetRawHandler {
    pub id: String,
}

impl JmapMessageGetRawHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<Vec<u8>> {
        let output = client.email_get(vec![self.id], None, true, false, 0)?;

        let email = output
            .emails
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Email not found"))?;

        Ok(email_to_raw(&email))
    }
}

fn email_to_raw(email: &Email) -> Vec<u8> {
    let mut msg = String::new();

    if let Some(from) = &email.from {
        if let Some(addr) = from.first() {
            msg.push_str(&format!("From: {}\r\n", format_addr(addr)));
        }
    }
    if let Some(to) = &email.to {
        let addrs: Vec<String> = to.iter().map(format_addr).collect();
        if !addrs.is_empty() {
            msg.push_str(&format!("To: {}\r\n", addrs.join(", ")));
        }
    }
    if let Some(cc) = &email.cc {
        let addrs: Vec<String> = cc.iter().map(format_addr).collect();
        if !addrs.is_empty() {
            msg.push_str(&format!("Cc: {}\r\n", addrs.join(", ")));
        }
    }
    if let Some(subject) = &email.subject {
        msg.push_str(&format!("Subject: {subject}\r\n"));
    }
    if let Some(date) = email.sent_at.as_ref().or(email.received_at.as_ref()) {
        msg.push_str(&format!("Date: {date}\r\n"));
    }
    if let Some(ids) = &email.message_id {
        if let Some(id) = ids.first() {
            msg.push_str(&format!("Message-ID: <{id}>\r\n"));
        }
    }
    if let Some(ids) = &email.in_reply_to {
        if let Some(id) = ids.first() {
            msg.push_str(&format!("In-Reply-To: <{id}>\r\n"));
        }
    }
    if let Some(refs) = &email.references {
        let s: Vec<String> = refs.iter().map(|r| format!("<{r}>")).collect();
        if !s.is_empty() {
            msg.push_str(&format!("References: {}\r\n", s.join(" ")));
        }
    }
    msg.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");

    if let (Some(text_body), Some(body_values)) = (&email.text_body, &email.body_values) {
        if let Some(part) = text_body.first() {
            if let Some(part_id) = &part.part_id {
                if let Some(bv) = body_values.get(part_id) {
                    msg.push_str(&bv.value);
                }
            }
        }
    }

    msg.into_bytes()
}

fn format_addr(addr: &EmailAddress) -> String {
    match &addr.name {
        Some(n) if !n.is_empty() => format!("{n} <{}>", addr.email),
        _ => addr.email.clone(),
    }
}
