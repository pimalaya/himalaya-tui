use std::collections::HashSet;

use anyhow::Result;
use io_jmap::{
    client::JmapClientStd,
    rfc8621::email::{EmailComparator, EmailFilter, EmailProperty},
};

use crate::app::Envelope;

pub struct JmapEnvelopeListHandler {
    pub mailbox_id: String,
    pub page: usize,
    pub page_size: usize,
}

impl JmapEnvelopeListHandler {
    pub fn execute(self, client: &mut JmapClientStd) -> Result<(Vec<Envelope>, u32)> {
        let filter = Some(EmailFilter {
            in_mailbox: Some(self.mailbox_id),
            ..Default::default()
        });
        let sort = Some(vec![EmailComparator::received_at_desc()]);
        let position = Some((self.page * self.page_size) as u64);
        let limit = Some(self.page_size as u64);
        let properties = Some(vec![
            EmailProperty::Id,
            EmailProperty::From,
            EmailProperty::Subject,
            EmailProperty::ReceivedAt,
            EmailProperty::Keywords,
        ]);

        let output = client.email_query(filter, sort, position, limit, properties)?;

        let envelopes = output
            .emails
            .into_iter()
            .map(|email| {
                let id = email.id.clone().unwrap_or_default();

                let from = email
                    .from
                    .as_deref()
                    .and_then(|a| a.first())
                    .map(|a| {
                        a.name
                            .as_deref()
                            .filter(|n| !n.is_empty())
                            .unwrap_or(&a.email)
                            .to_string()
                    })
                    .unwrap_or_default();

                let subject = email.subject.clone().unwrap_or_default();

                let date = email
                    .received_at
                    .as_deref()
                    .map(|s| s.get(..10).unwrap_or(s).to_string())
                    .unwrap_or_default();

                let flags = email
                    .keywords
                    .as_ref()
                    .map(|kw| {
                        kw.iter()
                            .filter_map(|(k, &v)| if v { Some(k.clone()) } else { None })
                            .collect::<HashSet<_>>()
                    })
                    .unwrap_or_default();

                Envelope {
                    id,
                    from,
                    subject,
                    date,
                    flags,
                }
            })
            .collect();

        Ok((envelopes, output.total.unwrap_or(0) as u32))
    }
}
