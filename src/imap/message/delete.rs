use anyhow::{bail, Result};
use io_imap::{
    client::ImapClientStd,
    types::{
        flag::{Flag, StoreType},
        sequence::SequenceSet,
    },
};
use pimalaya_stream::std::stream::StreamStd;

pub struct ImapMessageDeleteHandler {
    pub mailbox: String,
    pub id: String,
}

impl ImapMessageDeleteHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<()> {
        let mailbox_name = self.mailbox.try_into()?;
        let uid: u32 = self.id.parse()?;
        if uid == 0 {
            bail!("UID must be non-zero");
        }

        client.select(mailbox_name)?;

        let sequence_set: SequenceSet = uid.to_string().parse()?;

        client.store(sequence_set, StoreType::Add, vec![Flag::Deleted], true)?;

        Ok(())
    }
}
