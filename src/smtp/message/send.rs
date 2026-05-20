use std::{borrow::Cow, collections::HashSet};

use anyhow::{bail, Result};
use io_smtp::rfc5321::types::{
    domain::Domain, ehlo_domain::EhloDomain, forward_path::ForwardPath, local_part::LocalPart,
    mailbox::Mailbox, reverse_path::ReversePath,
};
use mail_parser::{Addr, Address, HeaderName, HeaderValue, MessageParser};

use crate::config::SmtpConfig;

pub struct SmtpMessageSendHandler {
    pub raw: Vec<u8>,
}

impl SmtpMessageSendHandler {
    pub fn execute(self, config: &SmtpConfig) -> Result<()> {
        let mut client = config.clone().into_client()?;
        let (reverse_path, forward_paths) = into_smtp_msg(&self.raw)?;

        client.send(reverse_path, forward_paths, self.raw)?;

        Ok(())
    }
}

fn into_smtp_msg(msg: &[u8]) -> Result<(ReversePath<'static>, Vec<ForwardPath<'static>>)> {
    let Some(msg) = MessageParser::new().parse_headers(msg) else {
        bail!("Invalid message to send")
    };

    let mut mail_from: Option<String> = None;
    let mut rcpt_to: HashSet<String> = HashSet::new();

    for header in msg.headers() {
        let key = &header.name;
        let val = header.value();

        match key {
            HeaderName::From => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    if let Some(email) = addrs.first().and_then(find_valid_email) {
                        mail_from = Some(email);
                    }
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    if let Some(group) = groups.first() {
                        if let Some(email) = group.addresses.first().and_then(find_valid_email) {
                            mail_from = Some(email);
                        }
                    }
                }
                _ => (),
            },
            HeaderName::To | HeaderName::Cc | HeaderName::Bcc => match val {
                HeaderValue::Address(Address::List(addrs)) => {
                    rcpt_to.extend(addrs.iter().filter_map(find_valid_email));
                }
                HeaderValue::Address(Address::Group(groups)) => {
                    rcpt_to.extend(
                        groups
                            .iter()
                            .flat_map(|group| group.addresses.iter())
                            .filter_map(find_valid_email),
                    );
                }
                _ => (),
            },
            _ => (),
        };
    }

    let Some(mail_from) = mail_from else {
        bail!("The message does not contain any sender");
    };

    if rcpt_to.is_empty() {
        bail!("The message does not contain any recipient");
    }

    let Some((local, domain)) = mail_from.split_once('@') else {
        bail!("The message contains an invalid sender");
    };

    let reverse_path = ReversePath::Mailbox(Mailbox {
        local_part: LocalPart(Cow::Owned(local.to_owned())),
        domain: EhloDomain::Domain(Domain(Cow::Owned(domain.to_owned()))),
    });

    let mut forward_paths = Vec::new();

    for rcpt in rcpt_to {
        let Some((local, domain)) = rcpt.split_once('@') else {
            bail!("The message contains an invalid recipient: {rcpt}");
        };

        forward_paths.push(ForwardPath(Mailbox {
            local_part: LocalPart(Cow::Owned(local.to_owned())),
            domain: EhloDomain::Domain(Domain(Cow::Owned(domain.to_owned()))),
        }));
    }

    Ok((reverse_path, forward_paths))
}

fn find_valid_email(addr: &Addr) -> Option<String> {
    match &addr.address {
        None => None,
        Some(email) => {
            let email = email.trim();
            if email.is_empty() {
                None
            } else {
                Some(email.to_string())
            }
        }
    }
}
