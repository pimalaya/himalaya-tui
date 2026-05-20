use anyhow::{anyhow, Result};
use io_imap::client::ImapClientStd;
use mail_parser::MessageParser;
use pimalaya_stream::std::stream::StreamStd;

use crate::imap::message::get_raw::ImapMessageGetRawHandler;

pub struct ImapMessageGetHandler {
    pub mailbox: String,
    pub id: String,
}

impl ImapMessageGetHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<String> {
        let raw = ImapMessageGetRawHandler {
            mailbox: self.mailbox,
            id: self.id,
        }
        .execute(client)?;

        let message = MessageParser::default()
            .parse(&raw)
            .ok_or_else(|| anyhow!("Failed to parse message"))?;

        let content = if let Some(text) = message.body_text(0) {
            text.to_string()
        } else if let Some(html) = message.body_html(0) {
            html2text::from_read(html.as_bytes(), 80)
        } else {
            String::from_utf8_lossy(&raw).to_string()
        };

        Ok(content)
    }
}
