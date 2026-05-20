use anyhow::Result;
use io_imap::{
    client::ImapClientStd,
    types::{core::Literal, extensions::binary::LiteralOrLiteral8, flag::Flag, mailbox::Mailbox},
};
use pimalaya_stream::std::stream::StreamStd;

pub struct ImapMessageSaveHandler {
    pub mailbox: String,
    pub raw: Vec<u8>,
    pub flags: Vec<Flag<'static>>,
}

impl ImapMessageSaveHandler {
    pub fn execute(self, client: &mut ImapClientStd<StreamStd>) -> Result<()> {
        let mailbox: Mailbox<'static> = self.mailbox.try_into()?;
        let literal = Literal::try_from(self.raw)?;
        let message = LiteralOrLiteral8::Literal(literal);

        client.append(mailbox, self.flags, None, message)?;

        Ok(())
    }
}
