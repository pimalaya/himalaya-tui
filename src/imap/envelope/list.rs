use anyhow::Result;
use io_imap::{
    client::ImapClientStd,
    types::{
        fetch::{MacroOrMessageDataItemNames, MessageDataItemName},
        sequence::SequenceSet,
    },
};
use pimalaya_stream::std::stream::StreamStd;

use crate::app::Envelope;
use crate::imap::util::parse_envelope;

pub struct ImapEnvelopeListHandler {
    pub mailbox: String,
    pub page: usize,
    pub page_size: usize,
}

impl ImapEnvelopeListHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<(Vec<Envelope>, u32)> {
        let mailbox_name = self.mailbox.try_into()?;

        let total = client.select(mailbox_name)?.exists.unwrap_or(0);

        if total == 0 {
            return Ok((Vec::new(), 0));
        }

        let page_size = self.page_size as u32;
        let offset = (self.page as u32) * page_size;

        if offset >= total {
            return Ok((Vec::new(), total));
        }

        // Sequence numbers are 1-based; newest messages have the
        // highest seq numbers.
        let end = total - offset;
        let start = end.saturating_sub(page_size - 1).max(1);

        let sequence_set: SequenceSet = format!("{start}:{end}").parse()?;
        let item_names = MacroOrMessageDataItemNames::MessageDataItemNames(vec![
            MessageDataItemName::Uid,
            MessageDataItemName::Envelope,
            MessageDataItemName::Flags,
        ]);

        let data = client.fetch(sequence_set, item_names, false)?;

        let mut envelopes: Vec<Envelope> = data
            .into_iter()
            .map(|(seq, items)| parse_envelope(seq.get(), items))
            .collect();

        envelopes.sort_by(|a, b| b.id.cmp(&a.id));

        Ok((envelopes, total))
    }
}
