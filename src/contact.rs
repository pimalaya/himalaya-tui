//! # Contacts
//!
//! Recipient completion backed by a command the user supplies, the aerc
//! `address-book-cmd` way: the TUI holds no address book of its own.
//!
//! [`ContactTarget`] locates the fragment to complete in the composer,
//! [`ContactLookup`] runs the command on a thread the event loop polls,
//! [`Contact`] parses and formats what it answers, and
//! [`ContactCompleteKey`] is the configurable key triggering it all.

use std::{
    fmt,
    io::Read,
    process::{Command, Stdio},
    str::FromStr,
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow, bail};
use pimalaya_config::command::{CommandConfig, shell};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// How long a contact command may run before it is killed.
const TIMEOUT: Duration = Duration::from_secs(5);

/// How often the lookup thread checks whether the command exited.
const WAIT_STEP: Duration = Duration::from_millis(10);

/// Header fields whose addresses complete, lowercased.
///
/// From is left out: it names the sender's own identity, which an
/// address book does not hold.
const FIELDS: [&str; 4] = ["to", "cc", "bcc", "reply-to"];

/// Placeholder a command line carries for the queried fragment.
const PLACEHOLDER: &str = "%s";

/// One contact a command answered.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contact {
    /// Email address.
    pub email: String,
    /// Display name, when the command gave one.
    pub name: Option<String>,
}

impl Contact {
    /// Parses a command's stdout, one contact per line.
    ///
    /// Each line is the address, then an optional tab and name, further
    /// tab-separated fields ignored (aerc's format). A line whose first
    /// field holds no `@` is skipped, which drops khard's header line.
    pub fn parse_all(stdout: &str) -> Vec<Self> {
        stdout
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let email = fields.next()?.trim();

                if !email.contains('@') {
                    return None;
                }

                let name = fields
                    .next()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(String::from);

                Some(Self {
                    email: email.to_owned(),
                    name,
                })
            })
            .collect()
    }
}

/// Renders the contact as an RFC 5322 mailbox, `Name <address>`.
///
/// The name is quoted (section 3.2.4) when it carries a special, so
/// `Doe, John` does not read as two recipients.
impl fmt::Display for Contact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(name) = &self.name else {
            return write!(f, "{}", self.email);
        };

        let special = |c: char| "()<>[]:;@\\,.\"".contains(c);

        if name.chars().any(special) {
            let escaped = name.replace('\\', "\\\\").replace('"', "\\\"");
            write!(f, "\"{escaped}\" <{}>", self.email)
        } else {
            write!(f, "{name} <{}>", self.email)
        }
    }
}

/// The fragment of a recipient header the cursor completes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactTarget {
    /// Row of the buffer the fragment sits on.
    pub row: usize,
    /// Column the fragment starts at, in chars.
    pub start: usize,
    /// Column the fragment ends at, the cursor's.
    pub end: usize,
    /// The fragment itself, sent to the command.
    pub query: String,
}

impl ContactTarget {
    /// Locates the fragment under the cursor, if it completes.
    ///
    /// It does when the cursor sits in the address part of a
    /// [`FIELDS`] header, continuation lines included, above the blank
    /// line ending the headers. The fragment runs from the last comma, or
    /// the colon, to the cursor, leading whitespace excluded.
    pub fn locate(lines: &[Vec<char>], row: usize, col: usize) -> Option<Self> {
        let is_blank = |line: &Vec<char>| line.iter().all(|c| c.is_whitespace());

        if lines.get(..=row)?.iter().any(is_blank) {
            return None;
        }

        let is_folded = |line: &Vec<char>| line.first().is_some_and(|c| *c == ' ' || *c == '\t');

        let header = (0..=row).rev().find(|r| !is_folded(&lines[*r]))?;
        let colon = lines[header].iter().position(|c| *c == ':')?;
        let name: String = lines[header][..colon].iter().collect();

        if !FIELDS.contains(&name.trim().to_lowercase().as_str()) {
            return None;
        }

        let line = &lines[row];
        let body = if row == header { colon + 1 } else { 0 };
        let end = col.min(line.len());

        if end < body {
            return None;
        }

        let mut start = line[body..end]
            .iter()
            .rposition(|c| *c == ',')
            .map_or(body, |i| body + i + 1);

        while start < end && line[start].is_whitespace() {
            start += 1;
        }

        Some(Self {
            row,
            start,
            end,
            query: line[start..end].iter().collect(),
        })
    }
}

/// A contact command running off the event loop.
pub struct ContactLookup {
    /// Where in the buffer the answer goes.
    pub target: ContactTarget,
    receiver: Receiver<Result<Vec<Contact>>>,
}

impl ContactLookup {
    /// Spawns `config` for `target`'s fragment on a thread of its own.
    pub fn spawn(config: &CommandConfig, target: ContactTarget) -> Self {
        let mut command = build_command(config, &target.query);
        let (sender, receiver) = mpsc::channel();

        log::debug!("spawn contact command");
        log::trace!("{command:?}");

        thread::spawn(move || {
            let _ = sender.send(run(&mut command));
        });

        Self { target, receiver }
    }

    /// The command's answer, once it has one.
    pub fn try_result(&self) -> Option<Result<Vec<Contact>>> {
        match self.receiver.try_recv() {
            Ok(result) => Some(result),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Err(anyhow!("Contact command thread died"))),
        }
    }
}

/// Builds the command querying `query`.
///
/// A [`PLACEHOLDER`] is replaced by the query, and without one the query
/// is appended as a last argument. In a shell line it is quoted, and so
/// is a placeholder the user already quoted, so aerc lines port as is.
fn build_command(config: &CommandConfig, query: &str) -> Command {
    match config {
        CommandConfig::Shell(line) => {
            let quoted = shell_quote(query);

            let line = if line.contains(PLACEHOLDER) {
                line.replace("'%s'", PLACEHOLDER)
                    .replace("\"%s\"", PLACEHOLDER)
                    .replace(PLACEHOLDER, &quoted)
            } else {
                format!("{line} {quoted}")
            };

            shell(&line)
        }
        CommandConfig::Argv { program, args } => {
            let mut command = Command::new(program);

            if args.iter().any(|arg| arg.contains(PLACEHOLDER)) {
                command.args(args.iter().map(|arg| arg.replace(PLACEHOLDER, query)));
            } else {
                command.args(args).arg(query);
            }

            command
        }
    }
}

/// Quotes `value` as one word for the platform shell.
fn shell_quote(value: &str) -> String {
    if cfg!(windows) {
        format!("\"{}\"", value.replace('"', ""))
    } else {
        format!("'{}'", value.replace('\'', r"'\''"))
    }
}

/// Runs the command to completion, or kills it at [`TIMEOUT`].
///
/// A non-zero exit with no output is no match when the code is 1, the
/// grep convention khard and abook follow, and an error otherwise.
fn run(command: &mut Command) -> Result<Vec<Contact>> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Cannot spawn contact command")?;

    let stdout = read_in_background(child.stdout.take());
    let stderr = read_in_background(child.stderr.take());

    let deadline = Instant::now() + TIMEOUT;

    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Contact command timed out after {}s", TIMEOUT.as_secs());
        }

        thread::sleep(WAIT_STEP);
    };

    let stdout = stdout.join().unwrap_or_default();
    let stdout = String::from_utf8_lossy(&stdout);

    log::debug!("contact command exited with {status}");
    log::trace!("{stdout}");

    if !status.success() && stdout.trim().is_empty() {
        if status.code() == Some(1) {
            return Ok(Vec::new());
        }

        let stderr = stderr.join().unwrap_or_default();
        let stderr = String::from_utf8_lossy(&stderr);
        let reason = stderr.lines().next().unwrap_or_default().trim();
        bail!("Contact command failed ({status}): {reason}");
    }

    Ok(Contact::parse_all(&stdout))
}

/// Drains a child pipe on a thread, so a full pipe never stalls it.
fn read_in_background(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

/// The key triggering completion, `tab` by default.
///
/// Written as a key name with optional `ctrl-`, `alt-` and `shift-`
/// prefixes: `tab`, `ctrl-space`, `alt-c`, `f2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactCompleteKey {
    /// The key itself.
    pub code: KeyCode,
    /// Modifiers held with it.
    pub modifiers: KeyModifiers,
}

impl ContactCompleteKey {
    /// Whether `key` is this one.
    ///
    /// Shift is ignored for a char and for backtab, terminals reporting
    /// it inconsistently there.
    pub fn matches(&self, key: &KeyEvent) -> bool {
        let lenient = matches!(self.code, KeyCode::Char(_) | KeyCode::BackTab);
        let strip = |m: KeyModifiers| {
            if lenient { m - KeyModifiers::SHIFT } else { m }
        };

        key.code == self.code && strip(key.modifiers) == strip(self.modifiers)
    }
}

impl Default for ContactCompleteKey {
    fn default() -> Self {
        Self {
            code: KeyCode::Tab,
            modifiers: KeyModifiers::NONE,
        }
    }
}

impl FromStr for ContactCompleteKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let lower = s.trim().to_lowercase();
        let mut parts: Vec<&str> = lower.split('-').collect();
        let key = parts.pop().unwrap_or_default();
        let mut modifiers = KeyModifiers::NONE;

        for part in parts {
            modifiers |= match part {
                "ctrl" | "control" => KeyModifiers::CONTROL,
                "alt" | "meta" => KeyModifiers::ALT,
                "shift" => KeyModifiers::SHIFT,
                _ => bail!("Invalid key `{s}`: unknown modifier `{part}`"),
            };
        }

        let mut chars = key.chars();
        let code = match key {
            "tab" if modifiers.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
            "tab" => KeyCode::Tab,
            "backtab" => {
                modifiers |= KeyModifiers::SHIFT;
                KeyCode::BackTab
            }
            "space" => KeyCode::Char(' '),
            _ => match (chars.next(), chars.next()) {
                (Some(c), None) => KeyCode::Char(c),
                (Some('f'), Some(_)) => match key[1..].parse() {
                    Ok(n @ 1..=12) => KeyCode::F(n),
                    _ => bail!("Invalid key `{s}`"),
                },
                _ => bail!("Invalid key `{s}`"),
            },
        };

        Ok(Self { code, modifiers })
    }
}

impl fmt::Display for ContactCompleteKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (modifier, name) in [
            (KeyModifiers::CONTROL, "ctrl-"),
            (KeyModifiers::ALT, "alt-"),
            (KeyModifiers::SHIFT, "shift-"),
        ] {
            if self.modifiers.contains(modifier) {
                f.write_str(name)?;
            }
        }

        match self.code {
            KeyCode::Tab | KeyCode::BackTab => f.write_str("tab"),
            KeyCode::Char(' ') => f.write_str("space"),
            KeyCode::Char(c) => write!(f, "{c}"),
            KeyCode::F(n) => write!(f, "f{n}"),
            code => write!(f, "{code}"),
        }
    }
}

impl<'de> Deserialize<'de> for ContactCompleteKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl Serialize for ContactCompleteKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use pimalaya_config::command::CommandConfig;
    use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use super::{Contact, ContactCompleteKey, ContactTarget, build_command, run};

    fn lines(text: &str) -> Vec<Vec<char>> {
        text.split('\n').map(|l| l.chars().collect()).collect()
    }

    fn contact(email: &str, name: Option<&str>) -> Contact {
        Contact {
            email: email.into(),
            name: name.map(Into::into),
        }
    }

    fn args(config: CommandConfig, query: &str) -> Vec<String> {
        build_command(&config, query)
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn parse_reads_aerc_format() {
        let out = "searching for 'bo' ...\nbob@example.com\tBob Smith\tother\nann@example.com\n\nnot-an-address\tNope\n";

        assert_eq!(
            Contact::parse_all(out),
            [
                contact("bob@example.com", Some("Bob Smith")),
                contact("ann@example.com", None),
            ]
        );
    }

    #[test]
    fn display_quotes_names_with_specials() {
        let bare = contact("a@x", None);
        let plain = contact("a@x", Some("Ann Lee"));
        let comma = contact("d@x", Some("Doe, \"J\""));

        assert_eq!(bare.to_string(), "a@x");
        assert_eq!(plain.to_string(), "Ann Lee <a@x>");
        assert_eq!(comma.to_string(), r#""Doe, \"J\"" <d@x>"#);
    }

    #[test]
    #[cfg(unix)]
    fn shell_line_quotes_the_query() {
        let q = "o'brien";
        let quoted = r"'o'\''brien'";

        let bare = args(CommandConfig::Shell("khard email %s".into()), q);
        let aerc = args(CommandConfig::Shell("khard email '%s'".into()), q);
        let none = args(CommandConfig::Shell("lookup".into()), q);

        assert_eq!(bare[1], format!("khard email {quoted}"));
        assert_eq!(aerc[1], format!("khard email {quoted}"));
        assert_eq!(none[1], format!("lookup {quoted}"));
    }

    /// Exit 1 with no output is khard's and abook's "no match", any
    /// other failure is an error worth showing.
    #[test]
    #[cfg(unix)]
    fn run_reads_exit_codes() {
        let run = |line: &str| run(&mut build_command(&CommandConfig::Shell(line.into()), "q"));

        let found = run("printf 'a@x\\tA\\n'; : %s").unwrap();
        let none = run(": %s; echo 'no match' >&2; exit 1").unwrap();
        let failed = run(": %s; echo 'boom' >&2; exit 2").unwrap_err();

        assert_eq!(found, [contact("a@x", Some("A"))]);
        assert!(none.is_empty());
        assert!(failed.to_string().ends_with("boom"), "{failed}");
    }

    #[test]
    fn argv_substitutes_or_appends() {
        let with = CommandConfig::Argv {
            program: "khard".into(),
            args: vec!["email".into(), "%s".into()],
        };
        let without = CommandConfig::Argv {
            program: "abook".into(),
            args: vec!["--mutt-query".into()],
        };

        assert_eq!(args(with, "o'b"), ["email", "o'b"]);
        assert_eq!(args(without, "o'b"), ["--mutt-query", "o'b"]);
    }

    #[test]
    fn locate_second_recipient() {
        let buf = lines("From: me@x\nTo: alice@x, bo\nSubject: hi\n\nbody");
        let target = ContactTarget::locate(&buf, 1, 15).unwrap();

        assert_eq!(target.query, "bo");
        assert_eq!((target.row, target.start, target.end), (1, 13, 15));
    }

    #[test]
    fn locate_folded_and_case_insensitive() {
        let buf = lines("CC: alice@x,\n  car\n\nbody");
        let target = ContactTarget::locate(&buf, 1, 5).unwrap();

        assert_eq!(target.query, "car");
        assert_eq!(target.start, 2);
    }

    #[test]
    fn locate_refuses_other_places() {
        let buf = lines("From: me\nTo: al\nSubject: x\n\nTo: body");

        assert_eq!(ContactTarget::locate(&buf, 0, 8), None);
        assert_eq!(ContactTarget::locate(&buf, 1, 1), None);
        assert_eq!(ContactTarget::locate(&buf, 2, 10), None);
        assert_eq!(ContactTarget::locate(&buf, 4, 8), None);
    }

    #[test]
    fn locate_empty_fragment() {
        let buf = lines("To: \n\n");
        let target = ContactTarget::locate(&buf, 0, 4).unwrap();

        assert_eq!(target.query, "");
    }

    #[test]
    fn key_parses_and_matches() {
        let tab: ContactCompleteKey = "tab".parse().unwrap();
        let ctrl_space: ContactCompleteKey = "Ctrl-Space".parse().unwrap();
        let shift_tab: ContactCompleteKey = "shift-tab".parse().unwrap();
        let f2: ContactCompleteKey = "f2".parse().unwrap();

        assert_eq!(tab, ContactCompleteKey::default());
        assert!(ctrl_space.matches(&KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL)));
        assert!(!ctrl_space.matches(&KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)));
        assert!(shift_tab.matches(&KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)));
        assert_eq!(f2.code, KeyCode::F(2));
        assert_eq!(ctrl_space.to_string(), "ctrl-space");
        assert!("hyper-x".parse::<ContactCompleteKey>().is_err());
        assert!("f13".parse::<ContactCompleteKey>().is_err());
    }
}
