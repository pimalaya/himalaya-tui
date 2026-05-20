use anyhow::Result;
use io_jmap::{client::JmapClientStd, rfc8621::email_set::JmapEmailSetArgs};

pub struct JmapFlagUpdateHandler {
    pub id: String,
    pub add: Vec<String>,
    pub remove: Vec<String>,
}

impl JmapFlagUpdateHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<()> {
        let mut args = JmapEmailSetArgs::default();

        for keyword in self.add {
            args.set_keyword(self.id.clone(), keyword);
        }
        for keyword in self.remove {
            args.unset_keyword(self.id.clone(), keyword);
        }

        client.email_set(args)?;

        Ok(())
    }
}
