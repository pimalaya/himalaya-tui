use anyhow::Result;
use io_jmap::client::JmapClientStd;

use crate::app::Mailbox;

pub struct JmapMailboxListHandler;

impl JmapMailboxListHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<Vec<Mailbox>> {
        let output = client.mailbox_query(None, None, None, None, None)?;

        let result = output
            .mailboxes
            .into_iter()
            .map(|m| Mailbox {
                id: m.id.clone(),
                name: m.name.clone().unwrap_or_default(),
                delimiter: None,
                subscribed: m.is_subscribed,
            })
            .collect();

        Ok(result)
    }
}
