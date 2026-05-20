use std::collections::BTreeMap;

use anyhow::{anyhow, bail, Result};
use io_jmap::{
    client::JmapClientStd,
    rfc8621::{
        email::EmailImport,
        email_submission::{EmailSubmissionCreate, Envelope},
    },
};

use crate::jmap::message::save::upload_url;

pub struct JmapMessageSendHandler {
    pub raw: Vec<u8>,
    /// ID of the Sent mailbox to store the outgoing message in.
    /// JMAP requires at least one mailbox for `Email/import`.
    pub sent_mailbox_id: Option<String>,
    /// Explicit SMTP envelope override. `None` means derive from
    /// message headers.
    pub envelope: Option<Envelope>,
}

impl JmapMessageSendHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<()> {
        let upload_url = upload_url(client)?;

        let blob = client.blob_upload(&upload_url, "message/rfc822", self.raw)?;

        let identity_id = client
            .identity_get(None)?
            .identities
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("No JMAP identities found"))?
            .id;

        // EmailSubmission/set requires an existing email ID; import the
        // blob into the Sent mailbox first (or bail if none is known).
        let mailbox_ids = match self.sent_mailbox_id {
            Some(id) => [(id, true)].into_iter().collect(),
            None => {
                bail!("No Sent mailbox ID available; cannot import message before JMAP submission")
            }
        };

        let import = EmailImport {
            blob_id: blob.blob_id,
            mailbox_ids,
            keywords: Some([("$seen".to_string(), true)].into_iter().collect()),
            received_at: None,
        };

        let mut emails: BTreeMap<String, EmailImport> = BTreeMap::new();
        emails.insert("send".to_string(), import);

        let imported = client.email_import(emails)?;

        if let Some(err) = imported.not_created.get("send") {
            bail!("JMAP Email/import failed: {err:?}");
        }

        let email_id = imported
            .created
            .get("send")
            .and_then(|e| e.id.clone())
            .ok_or_else(|| anyhow!("Email/import succeeded but no email ID returned"))?;

        let submission = EmailSubmissionCreate {
            identity_id,
            email_id,
            envelope: self.envelope,
        };

        let mut submissions: BTreeMap<String, EmailSubmissionCreate> = BTreeMap::new();
        submissions.insert("send".to_string(), submission);

        let submitted = client.email_submission_set(submissions)?;

        if let Some(err) = submitted.not_created.get("send") {
            bail!("JMAP EmailSubmission/set failed: {err:?}");
        }

        Ok(())
    }
}
