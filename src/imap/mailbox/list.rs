use anyhow::Result;
use io_imap::{client::ImapClientStd, types::mailbox::Mailbox as ImapMailbox};
use pimalaya_stream::std::stream::StreamStd;

use crate::app::Mailbox;

pub struct ImapMailboxListHandler;

impl ImapMailboxListHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<Vec<Mailbox>> {
        let reference = "".try_into()?;
        let pattern = "*".try_into()?;

        let mailboxes = client.lsub(reference, pattern)?;

        let result = mailboxes
            .into_iter()
            .map(|(mbox, delim, _attrs)| {
                let name = match mbox {
                    ImapMailbox::Inbox => "INBOX".to_string(),
                    ImapMailbox::Other(mbox) => {
                        String::from_utf8_lossy(mbox.inner().as_ref()).to_string()
                    }
                };
                let delimiter = delim.map(|d| d.inner());

                Mailbox {
                    id: None,
                    name,
                    delimiter,
                    subscribed: true,
                }
            })
            .collect();

        Ok(result)
    }
}
