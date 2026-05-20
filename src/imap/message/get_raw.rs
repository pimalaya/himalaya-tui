use anyhow::{anyhow, bail, Result};
use io_imap::{
    client::ImapClientStd,
    types::fetch::{MacroOrMessageDataItemNames, MessageDataItem},
};
use pimalaya_stream::std::stream::StreamStd;

use crate::imap::util::fetch_item_names_body_peek;

pub struct ImapMessageGetRawHandler {
    pub mailbox: String,
    pub id: String,
}

impl ImapMessageGetRawHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<Vec<u8>> {
        let mailbox_name = self.mailbox.try_into()?;
        let uid: u32 = self.id.parse()?;
        if uid == 0 {
            bail!("UID must be non-zero");
        }

        client.select(mailbox_name)?;

        let item_names =
            MacroOrMessageDataItemNames::MessageDataItemNames(fetch_item_names_body_peek());

        let sequence_set = uid.to_string().parse()?;
        let mut data = client.fetch(sequence_set, item_names, true)?;

        let (_, items) = data
            .pop_first()
            .ok_or_else(|| anyhow!("No message data returned"))?;

        for item in items {
            if let MessageDataItem::BodyExt { data, .. } = item {
                if let Some(data) = data.0 {
                    return Ok(data.as_ref().to_vec());
                }
            }
        }

        Err(anyhow!("No message data returned"))
    }
}
