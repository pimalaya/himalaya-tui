//! TOML configuration model loaded from the same file used by the
//! [`himalaya`] CLI. Each per-backend block deserializes here; the live
//! clients that consume it live in the per-protocol modules
//! (`crate::imap`, `crate::jmap`, …).
//!
//! `Config::project_name()` returns `"himalaya"` (not the crate name)
//! so the default XDG path resolves to `himalaya/config.toml`, allowing
//! the same file to back both binaries.
//!
//! Every block is modelled unconditionally, so one configuration file
//! loads whatever backends a build enables. The converters turning a
//! block into a runtime handle are therefore modelled unconditionally
//! too, and each carries an `allow(dead_code)` for the builds whose
//! feature set leaves it without a caller. They gate on no cargo
//! feature of their own, since the crates they need (io-sasl,
//! pimalaya-stream, url) are unconditional dependencies.
//!
//! [`himalaya`]: https://github.com/pimalaya/himalaya

use std::{collections::HashMap, path::PathBuf};

use anyhow::{Result, bail};
use io_sasl::{
    login::SaslLoginCreds, mechanism::Sasl, rfc4505::anonymous::SaslAnonymousCreds,
    rfc4616::plain::SaslPlainCreds, rfc5801::SaslGs2ChannelBinding, rfc5802::SaslScramCreds,
    rfc7628::oauthbearer::SaslOauthbearerCreds, xoauth2::SaslXoauth2Creds,
};
use pimalaya_config::{
    secret::Secret,
    toml::{TomlConfig, shell_expanded_string},
};
use pimalaya_stream::tls::{Rustls, RustlsCrypto, Tls, TlsProvider};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::tui::{
    model::Keybinds,
    theme::{self, Theme},
};

/// `skip_serializing_if` predicate skipping a field equal to its type's
/// default, so a wizard-generated account omits defaulted scalars (the
/// only serializer is the wizard, see [`crate::wizard`]).
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

/// `deny_unknown_fields` is intentionally omitted so the same TOML
/// file can be shared with the `himalaya` CLI: top-level CLI-only
/// sections (`table`, `envelope`, `mailbox`, `message`, `attachment`,
/// `account`) are silently ignored here.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,
    /// Signature appended to a composed draft when an account declares
    /// none of its own. See [`AccountConfig::signature`].
    pub signature: Option<String>,
    /// Separator written before the signature when an account declares
    /// none of its own. See [`AccountConfig::signature_delim`].
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    /// Composer keybinding flavor (Vim or Emacs). The CLI `--keybinds`
    /// flag overrides this; both default to Vim when omitted.
    pub keybinds: Option<Keybinds>,
    /// Color theme: pick a preset (`dracula-dark`, `one-light`,
    /// `tokyo-night`) and/or override individual fields. Resolved into
    /// a [`Theme`] at startup.
    #[serde(default)]
    pub theme: ThemeConfig,
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    /// Hard-coded to `"himalaya"` (not `CARGO_PKG_NAME`) so the TUI's
    /// default XDG path resolves to the same `himalaya/config.toml`
    /// the CLI uses, allowing one shared configuration file.
    fn project_name() -> &'static str {
        "himalaya"
    }

    /// Overrides the default implementation, whose bare "Get account
    /// error" leaves the user guessing: a name that does not exist is
    /// usually a typo, so the error lists the ones the file does hold.
    /// The default-account arm is unchanged, [`crate::cli`] turning its
    /// [`None`] into the wizard.
    fn take_account(&mut self, name: Option<&str>) -> Result<Option<(String, Self::Account)>> {
        let Some(name) = name.filter(|name| !name.is_empty() && *name != "default") else {
            return Ok(self.take_default_account());
        };

        if let Some(account) = self.take_named_account(name) {
            return Ok(Some(account));
        }

        let mut known: Vec<&str> = self.accounts.keys().map(String::as_str).collect();
        known.sort_unstable();

        if known.is_empty() {
            bail!("No account `{name}`: the configuration file declares none at all");
        }

        bail!("No account `{name}` in the configuration file, which declares {known:?}")
    }

    fn take_named_account(&mut self, name: &str) -> Option<(String, Self::Account)> {
        self.accounts.remove_entry(name)
    }

    fn take_default_account(&mut self) -> Option<(String, Self::Account)> {
        let name = self
            .accounts
            .iter()
            .find_map(|(name, account)| account.default.then(|| name.clone()))?;

        self.take_named_account(&name)
    }
}

/// User-supplied theme configuration: pick a preset and/or override
/// individual fields. Each override is merged on top of the preset
/// via [`Style::patch`], so users can change just one attribute
/// (e.g. only `fg`) and inherit the rest from the preset.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThemeConfig {
    /// Preset theme name. Each variant maps to one file under
    /// src/tui/theme/.
    pub preset: Option<PresetConfig>,
    pub header: Option<StyleConfig>,
    pub status_bar: Option<StyleConfig>,
    pub border_active: Option<StyleConfig>,
    pub border_inactive: Option<StyleConfig>,
    pub dialog_border: Option<StyleConfig>,
    pub cursor: Option<StyleConfig>,
    pub mailbox_current: Option<StyleConfig>,
    pub envelope_header: Option<StyleConfig>,
    pub envelope_seen: Option<StyleConfig>,
    pub envelope_unread: Option<StyleConfig>,
    pub message_body: Option<StyleConfig>,
    pub compose_text: Option<StyleConfig>,
    pub compose_cursor: Option<StyleConfig>,
    pub compose_selection: Option<StyleConfig>,
}

/// Names of presets shipped with the binary. Contributors add a preset
/// by dropping a new file under src/tui/theme/, declaring it in
/// src/tui/theme.rs, and adding a variant and a match arm here.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresetConfig {
    Default,
    DraculaDark,
    OneLight,
    TokyoNight,
}

impl PresetConfig {
    pub const fn theme(self) -> Theme {
        match self {
            PresetConfig::Default => theme::default::THEME,
            PresetConfig::DraculaDark => theme::dracula_dark::THEME,
            PresetConfig::OneLight => theme::one_light::THEME,
            PresetConfig::TokyoNight => theme::tokyo_night::THEME,
        }
    }
}

/// Config-side mirror of ratatui's [`Style`]. Field names follow the rest of
/// the config (kebab-case); `mod` is a list of [`ModifierConfig`] variants
/// (`["bold", "italic"]`).
///
/// Example:
///
/// ```toml
/// [theme.cursor]
/// fg = "magenta"
/// bg = "#222"
/// mod = ["bold", "italic"]
/// ```
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StyleConfig {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub r#mod: Vec<ModifierConfig>,
}

impl From<&StyleConfig> for Style {
    fn from(c: &StyleConfig) -> Self {
        let mut s = Style::new();

        if let Some(fg) = c.fg {
            s = s.fg(fg);
        }

        if let Some(bg) = c.bg {
            s = s.bg(bg);
        }

        let m = c
            .r#mod
            .iter()
            .copied()
            .fold(Modifier::empty(), |acc, m| acc | Modifier::from(m));

        s.add_modifier(m)
    }
}

/// Kebab-case mirror of ratatui's [`Modifier`] for user config. Each
/// variant maps 1:1 to a `Modifier::*` flag via [`From`].
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModifierConfig {
    Bold,
    Dim,
    Italic,
    Underlined,
    SlowBlink,
    RapidBlink,
    Reversed,
    Hidden,
    CrossedOut,
}

impl From<ModifierConfig> for Modifier {
    fn from(m: ModifierConfig) -> Self {
        match m {
            ModifierConfig::Bold => Modifier::BOLD,
            ModifierConfig::Dim => Modifier::DIM,
            ModifierConfig::Italic => Modifier::ITALIC,
            ModifierConfig::Underlined => Modifier::UNDERLINED,
            ModifierConfig::SlowBlink => Modifier::SLOW_BLINK,
            ModifierConfig::RapidBlink => Modifier::RAPID_BLINK,
            ModifierConfig::Reversed => Modifier::REVERSED,
            ModifierConfig::Hidden => Modifier::HIDDEN,
            ModifierConfig::CrossedOut => Modifier::CROSSED_OUT,
        }
    }
}

/// `deny_unknown_fields` is omitted so per-account CLI-only sections
/// (`table`, `envelope`, `mailbox`, `attachment`) coexist in the same
/// `[accounts.<name>]` block.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    #[serde(default, skip_serializing_if = "is_default")]
    pub default: bool,
    pub imap: Option<ImapConfig>,
    pub smtp: Option<SmtpConfig>,
    pub jmap: Option<JmapConfig>,
    pub maildir: Option<MaildirConfig>,
    /// Address this account sends as. Aliased to `email`, the
    /// spelling the himalaya CLI writes, the two binaries sharing one
    /// configuration file.
    ///
    /// Written back under the CLI spelling: the wizard here saves into
    /// the file the CLI authors, and one key spelled two ways depending
    /// on which binary wrote the block helps nobody.
    #[serde(alias = "email", rename(serialize = "email"))]
    pub from: Option<String>,
    /// Name that address carries. Aliased to `display-name`, the
    /// spelling the CLI writes; falls back to the global
    /// [`Config::display_name`]. Written back under the CLI spelling,
    /// as [`AccountConfig::from`] is.
    #[serde(alias = "display-name", rename(serialize = "display-name"))]
    pub from_name: Option<String>,
    /// Signature appended to a composed draft, after the body. Falls
    /// back to the global [`Config::signature`].
    ///
    /// The value is the signature alone: the separator before it is
    /// [`AccountConfig::signature_delim`]'s business, so a signature
    /// written for one binary reads the same in the other.
    pub signature: Option<String>,
    /// Separator written before the signature, defaulting to the
    /// RFC 3676 §4.3 `"-- \n"`. Falls back to the global
    /// [`Config::signature_delim`].
    ///
    /// Written verbatim, so a value meant to stand on its own line
    /// carries its own trailing newline.
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    /// Mailbox aliases mapping a friendly name to a backend-native id.
    ///
    /// Written by the wizard and read by the himalaya CLI, which needs
    /// them to address a mailbox without hand-editing ids. Inert here:
    /// the TUI resolves a mailbox name against the live listing before
    /// every dispatch, so it never has an id to alias.
    #[serde(default, skip_serializing_if = "is_default")]
    pub mailbox: MailboxConfig,
}

/// Per-account `mailbox.*` options.
///
/// `deny_unknown_fields` is omitted so the CLI-only `mailbox.list.*`
/// rendering options coexist under the same key.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MailboxConfig {
    /// Aliases keyed by friendly name, `inbox = "INBOX"` and friends.
    #[serde(default, rename = "alias", alias = "aliases")]
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub aliases: HashMap<String, String>,
}

/// The order the rendered account groups its keys in, most defining
/// first: what the account is, who it speaks for, then the backend it
/// reads from, the transport it sends over, and the mailboxes it
/// names. Matches the himalaya CLI's own table, one file being written
/// by both wizards.
///
/// A key outside this list still renders, after the ones listed, so a
/// field added to [`AccountConfig`] can never go missing from a
/// generated document just because nobody updated this table.
const RENDER_ORDER: [&str; 10] = [
    "default",
    "email",
    "display-name",
    "signature",
    "signature-delim",
    "imap",
    "jmap",
    "maildir",
    "smtp",
    "mailbox",
];

impl AccountConfig {
    /// Renders this account as an `[accounts.<name>]` block, ready to
    /// be written to a configuration file or appended to one.
    ///
    /// The serializer decides what is written, so a field left at its
    /// default is omitted and nothing has to be listed here twice.
    /// What this adds is reading order: the flattened dotted keys come
    /// out alphabetically, which buries `imap.server` under the
    /// credentials that authenticate against it, and runs every group
    /// together. The groups are reordered, `server` is lifted to the
    /// top of its own, and a blank line separates them.
    pub fn render(&self, name: &str) -> Result<String> {
        // NOTE: borrowed rather than built into a `Config`, which would
        // mean cloning the account. The emitter only looks for an
        // `accounts` table, so any shape carrying one will do.
        #[derive(Serialize)]
        struct AccountDocument<'a> {
            accounts: HashMap<&'a str, &'a AccountConfig>,
        }

        let document = AccountDocument {
            accounts: HashMap::from([(name, self)]),
        };
        let rendered = pimalaya_config::toml::to_string(&document)?;

        // The emitter writes the header itself, and everything below it
        // is one dotted key per line.
        let (header, body) = match rendered.split_once('\n') {
            Some((header, body)) => (header, body),
            None => return Ok(rendered),
        };

        let mut groups: Vec<(String, Vec<&str>)> = Vec::new();

        for line in body.lines().filter(|line| !line.trim().is_empty()) {
            let key = line.split(['.', ' ']).next().unwrap_or(line).to_string();

            match groups.iter_mut().find(|(name, _)| *name == key) {
                Some((_, lines)) => lines.push(line),
                None => groups.push((key, vec![line])),
            }
        }

        groups.sort_by_key(|(key, _)| {
            RENDER_ORDER
                .iter()
                .position(|known| known == key)
                .unwrap_or(RENDER_ORDER.len())
        });

        let mut document = format!("{header}\n");

        for (index, (key, mut lines)) in groups.into_iter().enumerate() {
            if index > 0 {
                document.push('\n');
            }

            // The endpoint is what the group is about, so it reads
            // first; the credentials and the quirks qualify it.
            let server = format!("{key}.server ");
            lines.sort_by_key(|line| !line.starts_with(&server));

            for line in lines {
                document.push_str(line);
                document.push('\n');
            }
        }

        Ok(document)
    }
}

/// Parses a `server` field into a [`Url`].
///
/// A bare `host:port` must be detected by the absence of `://`: the
/// URL parser would otherwise read it as `scheme:path` (for instance
/// `mail.example.com:993` parses as the scheme `mail.example.com`), so
/// any string without an explicit `://` is treated as an authority
/// under `default_scheme`. The resulting scheme is validated against
/// `allowed`.
#[allow(dead_code)]
pub fn parse_server(server: &str, default_scheme: &str, allowed: &[&str]) -> Result<Url> {
    let url = if server.contains("://") {
        Url::parse(server)?
    } else {
        Url::parse(&format!("{default_scheme}://{server}"))?
    };

    let scheme = url.scheme();

    if !allowed.contains(&scheme) {
        bail!("Invalid server scheme `{scheme}`: expected one of {allowed:?}");
    }

    Ok(url)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapConfig {
    /// IMAP server address. Either a bare authority
    /// (`imap.example.com[:port]`, treated as `imaps://<authority>`),
    /// or a full URL with `imap://` (cleartext, optional STARTTLS),
    /// `imaps://` (implicit TLS) or `unix://` (a local pre-authenticated
    /// socket proxy such as sirup).
    pub server: String,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: bool,
    /// ALPN protocol identifiers offered during the TLS handshake. Set
    /// to `[]` to skip ALPN negotiation entirely. Left unset, the
    /// default io-imap itself defines applies (`["imap"]`, RFC 7595).
    /// Only relevant for the rustls provider; `native-tls` ignores ALPN.
    #[serde(default)]
    pub alpn: Option<Vec<String>>,
    pub sasl: Option<SaslConfig>,
    /// RFC 4959 SASL-IR quirk. Left unset, follows the advertised
    /// `SASL-IR` capability; `false` waits for the server's
    /// continuation request rather than inlining credentials with
    /// `AUTHENTICATE`. Coremail (126.com, 163.com) advertises it
    /// falsely.
    #[serde(default, skip_serializing_if = "is_default")]
    pub sasl_ir: Option<bool>,
    /// RFC 2971 `ID` extension quirks. Some providers (notably
    /// mail.qq.com, fastmail) require an `ID` exchange straight after
    /// authentication; set `id.auto = true` to opt in.
    #[serde(default)]
    pub id: ImapIdConfig,
    /// RFC 5256 `SORT` extension config.
    #[serde(default)]
    pub sort: ImapSortConfig,
}

/// Per-account `imap.sort.*` options.
///
/// Accepted so a configuration file shared with the himalaya CLI
/// loads, but inert here: the TUI paginates a sequence-set window and
/// reverses it rather than issuing `SORT`, so it has no fallback to
/// choose between.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapSortConfig {
    /// Forces the CLI's SORT fallback on or off. Read by the CLI only.
    pub fallback: Option<bool>,
}

/// Per-account `imap.id.*` quirks.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdConfig {
    /// When `true`, the auth coroutine chains an `ID` round-trip
    /// after the tagged auth response. Default `false` skips ID
    /// entirely.
    #[serde(default, skip_serializing_if = "is_default")]
    pub auto: bool,

    /// Parameters sent with the auto-ID command. Empty (default)
    /// sends `ID NIL`. For each entry: `true` substitutes
    /// himalaya-tui's canned value for the well-known keys (`name`,
    /// `version`, `vendor`, `support-url`) or `NIL` for unknown keys;
    /// `false` always sends `NIL`. Keys absent from this map are not
    /// transmitted.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub fields: HashMap<String, bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmtpConfig {
    /// SMTP server address. Either a bare authority
    /// (`smtp.example.com[:port]`, treated as `smtps://<authority>`),
    /// or a full URL with `smtp://` (cleartext, optional STARTTLS),
    /// `smtps://` (implicit TLS) or `unix://` (a local pre-authenticated
    /// socket proxy such as sirup).
    pub server: String,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default, skip_serializing_if = "is_default")]
    pub starttls: bool,
    /// ALPN protocol identifiers offered during the TLS handshake. Set
    /// to `[]` to skip ALPN negotiation entirely. Left unset, the
    /// default io-smtp itself defines applies (`["smtp"]`, RFC 7595).
    /// Only relevant for the rustls provider; `native-tls` ignores ALPN.
    #[serde(default)]
    pub alpn: Option<Vec<String>>,
    pub sasl: Option<SaslConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct JmapConfig {
    /// JMAP server address. Either a bare authority for `/.well-known/jmap`
    /// discovery, or a full session-endpoint URL.
    pub server: String,
    #[serde(default)]
    pub tls: TlsConfig,
    /// ALPN protocol identifiers offered during the TLS handshake. Set
    /// to `[]` to skip ALPN negotiation entirely. Left unset, the
    /// default io-jmap itself defines applies (`["http/1.1"]`, JMAP
    /// riding on HTTP/1.1). Only relevant for the rustls provider;
    /// `native-tls` ignores ALPN.
    #[serde(default)]
    pub alpn: Option<Vec<String>>,
    pub auth: JmapAuthConfig,
    /// Identity id used when sending. Left unset, the first identity
    /// reported by `Identity/get` on the live session is used.
    pub identity_id: Option<String>,
    /// Drafts mailbox id used to stage a message before submission.
    /// Left unset, the mailbox whose role is `drafts` (RFC 8621
    /// section 2.1) is resolved from the live session.
    pub drafts_mailbox_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum JmapAuthConfig {
    Header(Secret),
    Bearer {
        token: Secret,
    },
    Basic {
        #[serde(deserialize_with = "shell_expanded_string")]
        username: String,
        password: Secret,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct MaildirConfig {
    /// Filesystem root holding the per-account Maildir tree. The
    /// directory itself must already exist (the wizard does not
    /// create it); each child mailbox is a `Maildir` (with the
    /// standard `cur`/`new`/`tmp` subdirs).
    pub root: PathBuf,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    pub provider: Option<TlsProviderConfig>,
    #[serde(default)]
    pub rustls: RustlsConfig,
    pub cert: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    Rustls,
    NativeTls,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    pub crypto: Option<RustlsCryptoConfig>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum RustlsCryptoConfig {
    Aws,
    Ring,
}

#[allow(dead_code)]
impl TlsConfig {
    /// Builds the runtime [`Tls`] handle the connect helpers expect.
    /// `alpn` is the protocol-level ALPN list (`["imap"]`, `["smtp"]`,
    /// `["http/1.1"]`); pass an empty vec to skip ALPN. The TOML
    /// schema never exposes `tls.rustls.alpn` directly: the
    /// per-protocol `*.alpn` field is folded in here.
    pub fn into_tls(self, alpn: Vec<String>) -> Tls {
        Tls {
            provider: self.provider.map(|p| match p {
                TlsProviderConfig::Rustls => TlsProvider::Rustls,
                TlsProviderConfig::NativeTls => TlsProvider::NativeTls,
            }),
            rustls: Rustls {
                crypto: self.rustls.crypto.map(|c| match c {
                    RustlsCryptoConfig::Aws => RustlsCrypto::Aws,
                    RustlsCryptoConfig::Ring => RustlsCrypto::Ring,
                }),
                alpn,
            },
            cert: self.cert,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum SaslConfig {
    Anonymous(SaslAnonymousConfig),
    Login(SaslLoginConfig),
    Plain(SaslPlainConfig),
    Oauthbearer(SaslOauthbearerConfig),
    Xoauth2(SaslXoauth2Config),
    #[serde(rename = "scram-sha-256")]
    ScramSha256(SaslScramSha256Config),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    #[serde(deserialize_with = "shell_expanded_string")]
    #[serde(alias = "username")]
    pub authcid: String,
    #[serde(alias = "password")]
    pub passwd: Secret,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslOauthbearerConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub token: Secret,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslXoauth2Config {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub token: Secret,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslScramSha256Config {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

#[allow(dead_code)]
impl SaslConfig {
    /// Resolves the SASL config into a runtime [`Sasl`]. `host` and
    /// `port` come from the live server URL; they are only used by
    /// OAUTHBEARER (echoed in the GS2 header) and ignored by every
    /// other mechanism.
    pub fn try_into_sasl(self, host: impl ToString, port: u16) -> Result<Sasl> {
        Ok(match self {
            SaslConfig::Anonymous(c) => Sasl::Anonymous(SaslAnonymousCreds { message: c.message }),
            SaslConfig::Login(c) => Sasl::Login(SaslLoginCreds {
                username: c.username,
                password: c.password.get()?,
            }),
            SaslConfig::Plain(c) => Sasl::Plain(SaslPlainCreds {
                authzid: c.authzid,
                authcid: c.authcid,
                passwd: c.passwd.get()?,
            }),
            SaslConfig::Oauthbearer(c) => Sasl::Oauthbearer(SaslOauthbearerCreds {
                username: c.username,
                host: host.to_string(),
                port,
                token: c.token.get()?,
            }),
            SaslConfig::Xoauth2(c) => Sasl::Xoauth2(SaslXoauth2Creds {
                username: c.username,
                token: c.token.get()?,
            }),
            // NOTE: an empty nonce means "draw one for me": the client
            // fills it before the exchange, an I/O-free coroutine having
            // no way to generate randomness itself.
            SaslConfig::ScramSha256(c) => Sasl::ScramSha256(SaslScramCreds {
                username: c.username,
                password: c.password.get()?,
                nonce: Vec::new(),
                channel_binding: SaslGs2ChannelBinding::Unsupported,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{Config, parse_server};

    /// Every option the himalaya CLI accepts in a block the TUI also
    /// models must load here, whether the TUI acts on it or not: the
    /// two binaries share one configuration file, and a
    /// `deny_unknown_fields` block rejecting a CLI-only option would
    /// leave that file usable by one binary only.
    #[test]
    fn cli_only_options_are_accepted() {
        let toml = r#"
            [accounts.example]
            default = true
            downloads-dir = "~/downloads"
            table.preset = "││──╞═╪╡┆    ┬┴┌┐└┘"
            envelope.list.datetime-fmt = "%F %R%:z"
            mailbox.alias.inbox = "INBOX"

            imap.server = "example.com"
            imap.alpn = ["imap"]
            imap.sasl-ir = false
            imap.sort.fallback = true

            smtp.server = "example.com"
            smtp.alpn = ["smtp"]

            jmap.server = "fastmail.com"
            jmap.alpn = ["http/1.1"]
            jmap.auth.bearer.token.raw = "***"
            jmap.identity-id = "I0123abc"
            jmap.drafts-mailbox-id = "M0123abc"
        "#;

        let mut config: Config = toml::from_str(toml).unwrap();
        let account = config.accounts.remove("example").unwrap();

        let imap = account.imap.unwrap();
        assert_eq!(imap.alpn.as_deref(), Some(["imap".to_string()].as_slice()));
        assert_eq!(imap.sasl_ir, Some(false));
        assert_eq!(imap.sort.fallback, Some(true));

        let smtp = account.smtp.unwrap();
        assert_eq!(smtp.alpn.as_deref(), Some(["smtp".to_string()].as_slice()));

        let jmap = account.jmap.unwrap();
        assert_eq!(
            jmap.alpn.as_deref(),
            Some(["http/1.1".to_string()].as_slice())
        );
        assert_eq!(jmap.identity_id.as_deref(), Some("I0123abc"));
        assert_eq!(jmap.drafts_mailbox_id.as_deref(), Some("M0123abc"));
    }

    /// A backend block this build does not model must be ignored rather
    /// than rejected: one file backs both binaries and every feature set,
    /// so a block the CLI writes, or one an older build wrote, cannot cost
    /// the account the backends it still carries.
    #[test]
    fn a_backend_block_this_build_drops_is_ignored() {
        let toml = r#"
            [accounts.example]
            default = true
            maildir.root = "/tmp/mail"

            gmail.token.raw = "***"
            m2dir.root = "/tmp/m2store"
        "#;

        let mut config: Config = toml::from_str(toml).unwrap();
        let account = config.accounts.remove("example").unwrap();

        assert_eq!(
            account.maildir.unwrap().root,
            PathBuf::from("/tmp/mail"),
            "the modelled backend survives the blocks around it",
        );
    }

    /// An omitted ALPN list must stay distinguishable from an explicit
    /// empty one: the first resolves to the protocol default at connect
    /// time, the second deliberately skips ALPN negotiation, and
    /// collapsing them changes how servers answer the handshake.
    #[test]
    fn omitted_alpn_differs_from_an_empty_one() {
        let toml = r#"
            [accounts.example]
            imap.server = "example.com"
            smtp.server = "example.com"
            smtp.alpn = []
        "#;

        let mut config: Config = toml::from_str(toml).unwrap();
        let account = config.accounts.remove("example").unwrap();

        assert_eq!(account.imap.unwrap().alpn, None);
        assert_eq!(account.smtp.unwrap().alpn, Some(Vec::new()));
    }

    /// A bare authority carrying a port must not be read as a URL: the
    /// scheme grammar accepts dots, so `mail.example.com:993` parses as
    /// the scheme `mail.example.com` with the path `993`, silently
    /// connecting nowhere.
    #[test]
    fn bare_authority_keeps_its_host_and_port() {
        let url = parse_server("mail.example.com:993", "imaps", &["imap", "imaps"]).unwrap();
        assert_eq!(url.scheme(), "imaps");
        assert_eq!(url.host_str(), Some("mail.example.com"));
        assert_eq!(url.port(), Some(993));
    }

    #[test]
    fn explicit_url_keeps_its_scheme() {
        let url = parse_server("imap://mail.example.com", "imaps", &["imap", "imaps"]).unwrap();
        assert_eq!(url.scheme(), "imap");
    }

    #[test]
    fn unlisted_scheme_is_rejected() {
        assert!(parse_server("ftp://mail.example.com", "imaps", &["imap", "imaps"]).is_err());
    }

    /// The himalaya CLI spells the identity `email` and `display-name`,
    /// and one file backs both binaries, so a config its wizard wrote
    /// must reach the same two fields the TUI composes from.
    #[test]
    fn the_identity_reads_under_the_cli_spelling() {
        let config: Config = toml::from_str(
            r#"
            display-name = "Alice"

            [accounts.example]
            email = "alice@example.org"
            display-name = "Alice at work"
            "#,
        )
        .expect("the CLI spelling must deserialize");

        let account = config.accounts.get("example").expect("the example account");

        assert_eq!(config.display_name.as_deref(), Some("Alice"));
        assert_eq!(account.from.as_deref(), Some("alice@example.org"));
        assert_eq!(account.from_name.as_deref(), Some("Alice at work"));
    }

    /// The shipped sample is the field reference the README points at,
    /// so an option renamed here without being renamed there ships a
    /// template that does not load. Uncommenting every line is not the
    /// point (many are deliberately exclusive alternatives): the point
    /// is that what the file does declare still matches the model.
    #[test]
    fn shipped_sample_config_loads() {
        let sample = include_str!("../config.sample.toml");
        let config: Config = toml::from_str(sample).expect("config.sample.toml must deserialize");

        let account = config
            .accounts
            .get("example")
            .expect("config.sample.toml must declare the example account");

        assert!(account.default);
        assert!(account.imap.is_some());
        assert!(account.smtp.is_some());
    }
}
