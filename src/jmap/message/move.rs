use std::collections::BTreeMap;

use anyhow::Result;
use io_jmap::{client::JmapClientStd, rfc8621::email_set::JmapEmailSetArgs};

pub struct JmapMessageMoveHandler {
    pub id: String,
    pub target_mailbox_id: String,
}

impl JmapMessageMoveHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();
        let new_mailbox_ids = BTreeMap::from([(self.target_mailbox_id, true)]);
        args.replace_mailbox_ids(self.id, new_mailbox_ids);

        client.email_set(args)?;

        Ok(())
    }
}
