use anyhow::{bail, Result};
use io_imap::{
    client::ImapClientStd,
    types::{mailbox::Mailbox, sequence::SequenceSet},
};
use pimalaya_stream::std::stream::StreamStd;

pub struct ImapMessageMoveHandler {
    pub mailbox: String,
    pub id: String,
    pub target: String,
}

impl ImapMessageMoveHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<()> {
        let mailbox_name = self.mailbox.try_into()?;
        let uid: u32 = self.id.parse()?;
        if uid == 0 {
            bail!("UID must be non-zero");
        }

        client.select(mailbox_name)?;

        let sequence_set: SequenceSet = uid.to_string().parse()?;
        let target_mailbox: Mailbox<'static> = self.target.try_into()?;

        client.r#move(sequence_set, target_mailbox, true)?;

        Ok(())
    }
}
