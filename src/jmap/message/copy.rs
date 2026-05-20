use anyhow::Result;
use io_jmap::{client::JmapClientStd, rfc8621::email_set::JmapEmailSetArgs};

pub struct JmapMessageCopyHandler {
    pub id: String,
    pub target_mailbox_id: String,
}

impl JmapMessageCopyHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();
        args.add_to_mailbox(self.id, self.target_mailbox_id);

        client.email_set(args)?;

        Ok(())
    }
}
