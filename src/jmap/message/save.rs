use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use io_jmap::{
    client::JmapClientStd,
    rfc8621::{capabilities, email::EmailImport},
};
use url::Url;

pub struct JmapMessageSaveHandler {
    pub mailbox_id: String,
    pub raw: Vec<u8>,
}

impl JmapMessageSaveHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<()> {
        let upload_url = upload_url(client)?;

        let blob = client.blob_upload(&upload_url, "message/rfc822", self.raw)?;

        let import = EmailImport {
            blob_id: blob.blob_id,
            mailbox_ids: [(self.mailbox_id, true)].into_iter().collect(),
            keywords: Some([("$draft".to_string(), true)].into_iter().collect()),
            received_at: None,
        };

        let mut emails: BTreeMap<String, EmailImport> = BTreeMap::new();
        emails.insert("draft".to_string(), import);

        let output = client.email_import(emails)?;

        if let Some(err) = output.not_created.get("draft") {
            bail!("JMAP Email/import (draft) failed: {err:?}");
        }

        Ok(())
    }
}

pub(crate) fn upload_url(client: &JmapClientStd) -> Result<Url> {
    let session = client
        .session()
        .ok_or_else(|| anyhow!("JMAP session not initialized"))?;
    let account_id = session
        .primary_accounts
        .get(capabilities::MAIL)
        .map(|s| s.as_str())
        .unwrap_or("");
    let url = session
        .upload_url
        .replace("{accountId}", account_id)
        .parse()?;
    Ok(url)
}
