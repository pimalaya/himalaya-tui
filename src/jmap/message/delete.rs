use anyhow::Result;
use io_jmap::{client::JmapClientStd, rfc8621::email_set::JmapEmailSetArgs};

pub struct JmapMessageDeleteHandler {
    pub id: String,
}

impl JmapMessageDeleteHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();
        args.destroy(self.id);

        client.email_set(args)?;

        Ok(())
    }
}
