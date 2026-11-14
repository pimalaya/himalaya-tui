//! # Client
//!
//! Every mail operation of the interface, run as one of himalaya's own
//! commands, in-process, over one himalaya session kept for the whole
//! run, as himalaya's REPL and MCP server do.
//!
//! A command is an argv this module builds, one element per value, parsed
//! by himalaya's grammar, so nothing is ever split or quoted as a shell
//! line would be. Its output lands in [`Captured`], a printer keeping the
//! JSON value it prints and the raw bytes `message read --raw` prints,
//! and is decoded into the [`crate::email`] types: the coupling is the
//! CLI's versioned `--json` contract, never himalaya's Rust types.
//!
//! A message to send or save travels through a private temporary file
//! the command reads, never inline: an inline message has its literal
//! `\n` sequences read as line breaks.

use std::{fmt, io::Write, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use clap::Parser;
use himalaya::{
    backend::Backend,
    cli::Command,
    session::{Session, SessionAccount},
};
use pimalaya_cli::{error::partial_outcome_of, output::Bytes, printer::Printer};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use tempfile::NamedTempFile;

use crate::email::{envelope::Envelope, mailbox::Mailbox};

/// The session every command of the run goes through.
pub struct MailClient {
    session: Session<'static>,
}

impl MailClient {
    /// Opens a session over the account `account` names in the files at
    /// `config_paths`, the default one when `None`.
    ///
    /// Nothing connects yet: the first command builds the client.
    pub fn new(config_paths: Vec<PathBuf>, account: Option<String>, backend: Backend) -> Self {
        // NOTE: the session borrows them for the whole run, which is the
        // whole process.
        let config_paths: &'static [PathBuf] = Vec::leak(config_paths);
        let name: Option<&'static str> = account.map(|name| &*String::leak(name));

        let session = Session {
            account: SessionAccount {
                config_paths,
                name,
                backend,
            },
            ..Default::default()
        };

        Self { session }
    }

    /// Whether the client is built, so the next command does not connect
    /// first.
    pub fn is_connected(&self) -> bool {
        self.session.is_connected()
    }

    /// Keeps the connection alive after an idle period.
    pub fn keep_alive(&mut self) -> Result<()> {
        self.session.keep_alive()
    }

    /// The account's mailboxes.
    pub fn list_mailboxes(&mut self) -> Result<Vec<Mailbox>> {
        let listing: MailboxListing = self.run(["mailbox", "list"])?.decode()?;
        Ok(listing.mailboxes)
    }

    /// One page of `mailbox`, the first being 1.
    pub fn list_envelopes(
        &mut self,
        mailbox: &str,
        page: usize,
        size: usize,
    ) -> Result<EnvelopePage> {
        let page = page.to_string();
        let size = size.to_string();
        let argv = ["envelope", "list", "-m", mailbox, "-p", &page, "-s", &size];
        self.run(argv)?.decode()
    }

    /// The raw RFC 5322 bytes of a message, its flags left alone.
    pub fn read_message(&mut self, mailbox: &str, id: &str) -> Result<Vec<u8>> {
        let argv = ["message", "read", "-m", mailbox, "--raw", "--", id];
        self.run(argv)?
            .bytes
            .context("himalaya printed no message bytes")
    }

    /// Copies messages from one mailbox to another.
    pub fn copy_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Receipt> {
        let argv = [&["message", "copy", "-f", from, "-t", to, "--"], ids].concat();
        self.run(argv)?.decode()
    }

    /// Moves messages from one mailbox to another.
    pub fn move_messages(&mut self, from: &str, to: &str, ids: &[&str]) -> Result<Receipt> {
        let argv = [&["message", "move", "-f", from, "-t", to, "--"], ids].concat();
        self.run(argv)?.decode()
    }

    /// Deletes messages, trash first: they move to the trash, or are
    /// removed for good when they are already there.
    pub fn delete_messages(&mut self, mailbox: &str, ids: &[&str]) -> Result<DeleteReceipt> {
        let argv = [&["message", "delete", "-m", mailbox, "--"], ids].concat();
        self.run(argv)?.decode()
    }

    /// Adds or removes one flag, named as `flag add` takes it.
    pub fn flag_messages(
        &mut self,
        mailbox: &str,
        flag: &str,
        add: bool,
        ids: &[&str],
    ) -> Result<Receipt> {
        let verb = if add { "add" } else { "remove" };
        let argv = [&["flag", verb, "-m", mailbox, "-f", flag, "--"], ids].concat();
        self.run(argv)?.decode()
    }

    /// Sends a MIME message, copied where `message.send.save-copy` says.
    pub fn send_message(&mut self, mime: &[u8]) -> Result<SendReceipt> {
        let file = message_file(mime)?;
        let path = file_path(&file)?;
        self.run(["message", "send", "--", path])?.decode()
    }

    /// Saves a draft, as it is, to the account's drafts mailbox.
    pub fn save_draft(&mut self, raw: &[u8]) -> Result<SendReceipt> {
        let file = message_file(raw)?;
        let path = file_path(&file)?;
        let argv = ["message", "add", "-m", "drafts", "-f", "draft", "--", path];
        self.run(argv)?.decode()
    }

    /// Runs one command over the session.
    ///
    /// A partial outcome is a result: the command printed it, its
    /// `errors` naming what failed. Any other error is a failure.
    fn run<'a>(&mut self, argv: impl IntoIterator<Item = &'a str>) -> Result<Captured> {
        let argv: Vec<&str> = argv.into_iter().collect();
        log::debug!("run himalaya {}", argv.join(" "));

        let line = Line::try_parse_from(argv).map_err(|err| anyhow!("{err}"))?;
        let mut printer = Captured::default();

        match line.cmd.execute(&mut printer, &mut self.session) {
            Ok(()) => Ok(printer),
            Err(err) if partial_outcome_of(&err).is_some() => Ok(printer),
            Err(err) => Err(err),
        }
    }
}

/// One command line, the grammar of the CLI without its binary name and
/// global flags, the session fixing the account.
#[derive(Debug, Parser)]
#[command(no_binary_name = true)]
struct Line {
    #[command(subcommand)]
    cmd: Command,
}

/// What a command printed: its JSON value and the bytes of a raw read.
#[derive(Debug, Default)]
struct Captured {
    value: Option<Value>,
    bytes: Option<Vec<u8>>,
}

impl Captured {
    /// The printed value decoded as `T`.
    fn decode<T: DeserializeOwned>(self) -> Result<T> {
        let value = self.value.context("himalaya printed no result")?;
        serde_json::from_value(value).context("Decode himalaya's answer")
    }
}

impl Printer for Captured {
    fn out<T: fmt::Display + Serialize>(&mut self, data: T) -> Result<()> {
        self.value = Some(serde_json::to_value(&data)?);
        Ok(())
    }

    fn event<T: fmt::Display + Serialize>(&mut self, event: T) -> Result<()> {
        log::debug!("himalaya event: {}", serde_json::to_value(&event)?);
        Ok(())
    }

    fn out_bytes(&mut self, bytes: Bytes) -> Result<()> {
        self.bytes = Some(bytes.data);
        Ok(())
    }

    fn is_json(&self) -> bool {
        true
    }
}

/// The `mailbox list` answer.
#[derive(Debug, Deserialize)]
struct MailboxListing {
    mailboxes: Vec<Mailbox>,
}

/// The `envelope list` answer: one page.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvelopePage {
    /// The envelopes of the page, newest first.
    pub envelopes: Vec<Envelope>,
    /// The next page, `None` once a page came back short.
    pub next_page: Option<String>,
    /// How many messages the mailbox holds, `None` when the backend does
    /// not say within the listing.
    #[serde(default)]
    pub total: Option<u64>,
    /// The envelopes the page could not list.
    #[serde(default)]
    pub errors: Vec<ItemError>,
}

/// The answer of a write acting on messages by id.
#[derive(Debug, Deserialize)]
pub struct Receipt {
    /// How many messages it acted on.
    pub count: usize,
    /// Whether a sync engine applies it later (pimdir).
    #[serde(default)]
    pub queued: bool,
    /// The messages it could not act on, in the order given.
    #[serde(default)]
    pub errors: Vec<ItemError>,
}

/// The answer of `message delete`.
#[derive(Debug, Deserialize)]
pub struct DeleteReceipt {
    /// What the delete did.
    pub action: DeleteAction,
    /// How many messages it acted on, and those it could not.
    #[serde(flatten)]
    pub receipt: Receipt,
}

/// What `message delete` did to the messages it was given.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DeleteAction {
    /// They were moved to the trash.
    MovedToTrash,
    /// They were in the trash and were removed for good.
    Deleted,
    /// They were in the trash and were flagged `\Deleted`, an expunge
    /// still owing (IMAP without UIDPLUS).
    Flagged,
}

/// The answer of `message send` and `message add`.
#[derive(Debug, Deserialize)]
pub struct SendReceipt {
    /// Whether a copy was saved.
    #[serde(default)]
    pub saved: bool,
    /// Whether a sync engine sends or saves it later (pimdir).
    #[serde(default)]
    pub queued: bool,
}

/// The error of one item a command could not act on.
#[derive(Clone, Debug, Deserialize)]
pub struct ItemError {
    /// The item's id, `None` when unknown.
    pub id: Option<String>,
    /// The error message.
    pub error: String,
}

impl fmt::Display for ItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.id {
            Some(id) => write!(f, "{id}: {}", self.error),
            None => f.write_str(&self.error),
        }
    }
}

/// A private temporary file holding `contents`, removed when dropped.
fn message_file(contents: &[u8]) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new().context("Create a temporary message file")?;
    file.write_all(contents)
        .context("Write the temporary message file")?;
    Ok(file)
}

/// The path of `file` as an argv element.
fn file_path(file: &NamedTempFile) -> Result<&str> {
    file.path()
        .to_str()
        .context("The temporary directory's path is not UTF-8")
}

#[cfg(all(test, feature = "maildir"))]
mod tests {
    use std::fs;

    use himalaya::backend::Backend;
    use tempfile::TempDir;

    use super::MailClient;

    /// A Maildir account whose inbox holds `messages`, with an `Archive`
    /// and a `Drafts` folder, the latter aliased as the drafts mailbox.
    fn account(messages: &[(&str, &str)]) -> (TempDir, MailClient) {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("mail");

        for folder in [
            root.clone(),
            root.join("Archive"),
            root.join("Drafts"),
            root.join("Old projects"),
        ] {
            for sub in ["cur", "new", "tmp"] {
                fs::create_dir_all(folder.join(sub)).unwrap();
            }
        }

        for (id, subject) in messages {
            let message = format!("From: a@localhost\r\nSubject: {subject}\r\n\r\nbody\r\n");
            fs::write(root.join("cur").join(format!("{id}:2,")), message).unwrap();
        }

        let config = dir.path().join("config.toml");
        let toml = format!(
            "[accounts.test]\ndefault = true\nmaildir.root = {:?}\nmailbox.alias.drafts = \"Drafts\"\n",
            root.display().to_string(),
        );
        fs::write(&config, toml).unwrap();

        let client = MailClient::new(vec![config], None, Backend::Auto);
        (dir, client)
    }

    fn ids(client: &mut MailClient, mailbox: &str) -> Vec<String> {
        let page = client.list_envelopes(mailbox, 1, 50).unwrap();
        page.envelopes.into_iter().map(|e| e.id).collect()
    }

    #[test]
    fn a_page_reports_the_mailbox_total_and_the_next_page() {
        let (_dir, mut client) = account(&[("1", "One"), ("2", "Two"), ("3", "Three")]);

        let names: Vec<String> = client
            .list_mailboxes()
            .unwrap()
            .into_iter()
            .map(|mailbox| mailbox.name)
            .collect();
        assert!(names.iter().any(|name| name == "Archive"), "{names:?}");

        let page = client.list_envelopes("INBOX", 1, 2).unwrap();
        assert_eq!(page.envelopes.len(), 2);
        assert_eq!(page.total, Some(3));
        assert_eq!(page.next_page.as_deref(), Some("2"));
        assert!(client.is_connected());
    }

    #[test]
    fn a_raw_read_returns_the_message_bytes() {
        let (_dir, mut client) = account(&[("1", "Hello")]);
        let id = ids(&mut client, "INBOX").remove(0);

        let raw = client.read_message("INBOX", &id).unwrap();
        assert!(String::from_utf8_lossy(&raw).contains("Subject: Hello"));
    }

    #[test]
    fn a_partial_move_is_a_result_naming_what_failed() {
        let (_dir, mut client) = account(&[("1", "One")]);
        let id = ids(&mut client, "INBOX").remove(0);

        let receipt = client
            .move_messages("INBOX", "Archive", &[&id, "missing"])
            .unwrap();

        assert_eq!(receipt.count, 1);
        assert_eq!(receipt.errors.len(), 1);
        assert_eq!(receipt.errors[0].id.as_deref(), Some("missing"));
        assert_eq!(ids(&mut client, "Archive").len(), 1);
    }

    #[test]
    fn a_copy_reaches_a_mailbox_whose_name_holds_a_space() {
        let (_dir, mut client) = account(&[("1", "One")]);
        let id = ids(&mut client, "INBOX").remove(0);

        let receipt = client
            .copy_messages("INBOX", "Old projects", &[&id])
            .unwrap();

        assert_eq!(receipt.count, 1);
        assert_eq!(ids(&mut client, "Old projects").len(), 1);
        assert_eq!(ids(&mut client, "INBOX").len(), 1);
    }

    #[test]
    fn a_flag_removed_leaves_the_next_listing() {
        let (_dir, mut client) = account(&[("1", "One")]);
        let id = ids(&mut client, "INBOX").remove(0);
        client.flag_messages("INBOX", "seen", true, &[&id]).unwrap();

        let receipt = client
            .flag_messages("INBOX", "seen", false, &[&id])
            .unwrap();
        assert_eq!(receipt.count, 1);

        let page = client.list_envelopes("INBOX", 1, 10).unwrap();
        assert!(
            page.envelopes[0].flags.is_empty(),
            "{:?}",
            page.envelopes[0].flags
        );
    }

    #[test]
    fn a_flag_added_shows_on_the_next_listing() {
        let (_dir, mut client) = account(&[("1", "One")]);
        let id = ids(&mut client, "INBOX").remove(0);

        let receipt = client
            .flag_messages("INBOX", "flagged", true, &[&id])
            .unwrap();
        assert_eq!(receipt.count, 1);

        let page = client.list_envelopes("INBOX", 1, 10).unwrap();
        let flags: Vec<&str> = page.envelopes[0].flags.iter().map(|f| f.raw()).collect();
        assert!(
            flags.iter().any(|f| f.eq_ignore_ascii_case("\\Flagged")),
            "{flags:?}"
        );
    }

    /// The message goes through a file: inline, himalaya would read its
    /// literal `\n` as a line break.
    #[test]
    fn a_draft_is_saved_byte_for_byte_to_the_drafts_alias() {
        let (_dir, mut client) = account(&[]);
        let draft = "From: a@localhost\nSubject: Code\n\nprintf(\"\\n\");\n";

        client.save_draft(draft.as_bytes()).unwrap();

        let id = ids(&mut client, "drafts").remove(0);
        let raw = client.read_message("drafts", &id).unwrap();
        let raw = String::from_utf8_lossy(&raw);
        assert!(raw.contains("printf(\"\\n\");"), "{raw}");
    }

    #[test]
    fn a_failed_command_drops_the_client() {
        let (_dir, mut client) = account(&[("1", "One")]);
        client.list_mailboxes().unwrap();
        assert!(client.is_connected());

        assert!(client.read_message("INBOX", "missing").is_err());
        assert!(!client.is_connected());
    }

    #[test]
    fn no_configuration_fails_naming_the_path() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("missing.toml");
        let mut client = MailClient::new(vec![path.clone()], None, Backend::Auto);

        let err = client.list_mailboxes().unwrap_err();
        assert!(
            format!("{err:#}").contains(&path.display().to_string()),
            "{err:#}"
        );
    }
}
