// This file is part of Himalaya TUI, a TUI to manage emails.
//
// Copyright (C) 2025-2026  soywod <pimalaya.org@posteo.net>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

//! TOML configuration model loaded from the same file used by the
//! [`himalaya`] CLI, plus the `into_client` adapters that turn each
//! per-backend block into a live IMAP/SMTP/JMAP/Maildir client.
//!
//! `Config::project_name()` returns `"himalaya"` (not the crate name)
//! so the default XDG path resolves to `himalaya/config.toml`, allowing
//! the same file to back both binaries.
//!
//! [`himalaya`]: https://github.com/pimalaya/himalaya

use std::{collections::HashMap, fs, path::Path, path::PathBuf};

use anyhow::{Context, Result};
use pimalaya_config::{
    secret::{Secret, SecretError},
    toml::{TomlConfig, shell_expanded_string},
};
use pimalaya_stream::{
    sasl::{
        Sasl, SaslAnonymous, SaslLogin, SaslOauthbearer, SaslPlain, SaslScramSha256, SaslXoauth2,
    },
    tls::{Rustls, RustlsCrypto, Tls, TlsProvider},
};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "imap", feature = "smtp", feature = "jmap"))]
use url::Url;

use crate::{app::keybinds::Keybinds, theme::Theme, themes};

/// `deny_unknown_fields` is intentionally omitted so the same TOML
/// file can be shared with the `himalaya` CLI: top-level CLI-only
/// sections (`table`, `envelope`, `mailbox`, `message`, `attachment`,
/// `account`) are silently ignored here.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    #[serde(alias = "from-name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    /// Composer keybinding flavor (Vim or Emacs). The CLI `--keybinds`
    /// flag overrides this; both default to Vim when omitted.
    pub keybinds: Option<Keybinds>,
    /// Color theme: pick a preset (`dracula`, `one-dark`, ...) and/or
    /// override individual fields. Resolved into a [`Theme`] at
    /// startup.
    #[serde(default)]
    pub theme: ThemeConfig,
    pub accounts: HashMap<String, AccountConfig>,
}

/// User-supplied theme configuration: pick a preset and/or override
/// individual fields. Each override is merged on top of the preset
/// via [`Style::patch`], so users can change just one attribute
/// (e.g. only `fg`) and inherit the rest from the preset.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThemeConfig {
    /// Preset theme name. Each variant maps to one file under
    /// `src/themes/`.
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

/// Names of presets shipped with the binary. Contributors add a
/// preset by dropping a new file under `src/themes/`, registering it
/// in `src/themes/mod.rs`, and adding a variant + match arm here.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresetConfig {
    Default,
    DraculaDark,
    OneLight,
}

impl PresetConfig {
    pub const fn theme(self) -> Theme {
        match self {
            PresetConfig::Default => themes::default::THEME,
            PresetConfig::DraculaDark => themes::dracula_dark::THEME,
            PresetConfig::OneLight => themes::one_light::THEME,
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

impl Config {
    /// Serializes `self` to TOML and writes it to `path`, creating
    /// any missing parent directories. Used by the wizard to persist
    /// a freshly-built configuration.
    pub fn write(&self, path: &Path) -> Result<()> {
        let toml = toml::to_string_pretty(self).context("Serialize TOML config error")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Create TOML config parent `{}` error", parent.display())
            })?;
        }

        fs::write(path, toml)
            .with_context(|| format!("Write TOML config `{}` error", path.display()))?;

        Ok(())
    }
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    /// Hard-coded to `"himalaya"` (not `CARGO_PKG_NAME`) so the TUI's
    /// default XDG path resolves to the same `himalaya/config.toml`
    /// the CLI uses, allowing one shared configuration file.
    fn project_name() -> &'static str {
        "himalaya"
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

/// `deny_unknown_fields` is omitted so per-account CLI-only sections
/// (`table`, `envelope`, `mailbox`, `attachment`) coexist in the same
/// `[accounts.<name>]` block.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    #[serde(default)]
    pub default: bool,
    pub imap: Option<ImapConfig>,
    pub smtp: Option<SmtpConfig>,
    pub jmap: Option<JmapConfig>,
    pub maildir: Option<MaildirConfig>,
    pub m2dir: Option<M2dirConfig>,
    pub from: Option<String>,
    pub from_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapConfig {
    /// IMAP server address. Either a bare authority
    /// (`imap.example.com[:port]`, treated as `imaps://<authority>`),
    /// or a full URL with `imap://` (cleartext, optional STARTTLS) or
    /// `imaps://` (implicit TLS).
    pub server: String,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub starttls: bool,
    pub sasl: Option<SaslConfig>,
}

#[cfg(feature = "imap")]
impl ImapConfig {
    pub fn into_client(
        self,
    ) -> Result<io_imap::client::ImapClientStd<pimalaya_stream::std::stream::StreamStd>> {
        let mut tls: Tls = self.tls.try_into()?;
        tls.rustls.alpn = vec!["imap".into()];
        let sasl: Option<Sasl> = self.sasl.map(Sasl::try_from).transpose()?;
        let server = parse_imap_server(&self.server)?;
        Ok(io_imap::client::ImapClientStd::connect(
            &server,
            &tls,
            self.starttls,
            sasl,
        )?)
    }
}

#[cfg(feature = "imap")]
pub fn parse_imap_server(server: &str) -> Result<Url> {
    match Url::parse(server) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(Url::parse(&format!("imaps://{server}"))?)
        }
        Err(err) => Err(err.into()),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SmtpConfig {
    /// SMTP server address. Either a bare authority
    /// (`smtp.example.com[:port]`, treated as `smtps://<authority>`),
    /// or a full URL with `smtp://` (cleartext, optional STARTTLS) or
    /// `smtps://` (implicit TLS).
    pub server: String,
    #[serde(default)]
    pub tls: TlsConfig,
    #[serde(default)]
    pub starttls: bool,
    pub sasl: Option<SaslConfig>,
}

#[cfg(feature = "smtp")]
impl SmtpConfig {
    pub fn into_client(
        self,
    ) -> Result<io_smtp::client::SmtpClientStd<pimalaya_stream::std::stream::StreamStd>> {
        use std::net::Ipv4Addr;

        use io_smtp::rfc5321::types::ehlo_domain::EhloDomain;

        let mut tls: Tls = self.tls.try_into()?;
        tls.rustls.alpn = vec!["smtp".into()];
        let sasl: Option<Sasl> = self.sasl.map(Sasl::try_from).transpose()?;
        let domain: EhloDomain<'static> = Ipv4Addr::new(127, 0, 0, 1).into();
        let server = parse_smtp_server(&self.server)?;
        Ok(io_smtp::client::SmtpClientStd::connect(
            &server,
            &tls,
            self.starttls,
            domain,
            sasl,
        )?)
    }
}

#[cfg(feature = "smtp")]
pub fn parse_smtp_server(server: &str) -> Result<Url> {
    match Url::parse(server) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(Url::parse(&format!("smtps://{server}"))?)
        }
        Err(err) => Err(err.into()),
    }
}

/// `deny_unknown_fields` is omitted so CLI-only JMAP fields
/// (`identity-id`, `drafts-mailbox-id`) survive when the same block
/// is reused by the CLI.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct JmapConfig {
    /// JMAP server address. Either a bare authority for `/.well-known/jmap`
    /// discovery, or a full session-endpoint URL.
    pub server: String,
    #[serde(default)]
    pub tls: TlsConfig,
    pub auth: JmapAuthConfig,
}

#[cfg(feature = "jmap")]
impl JmapConfig {
    pub fn into_client(self) -> Result<io_jmap::client::JmapClientStd> {
        let mut tls: Tls = self.tls.try_into()?;
        tls.rustls.alpn = vec!["http/1.1".into()];

        let http_auth = jmap_http_auth(self.auth)?;
        let url = parse_jmap_server(&self.server)?;

        let mut client = io_jmap::client::JmapClientStd::connect(&url, &tls, http_auth)?;
        client.session_get(&url)?;
        Ok(client)
    }
}

#[cfg(feature = "jmap")]
pub fn parse_jmap_server(server: &str) -> Result<Url> {
    match Url::parse(server) {
        Ok(url) => Ok(url),
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            Ok(Url::parse(&format!("https://{server}"))?)
        }
        Err(err) => Err(err.into()),
    }
}

#[cfg(feature = "jmap")]
pub fn jmap_http_auth(config: JmapAuthConfig) -> Result<secrecy::SecretString> {
    use base64::{Engine, prelude::BASE64_STANDARD};
    use secrecy::ExposeSecret;

    match config {
        JmapAuthConfig::Header(token) => Ok(token.get()?),
        JmapAuthConfig::Bearer { token } => {
            let token = token.get()?;
            Ok(format!("Bearer {}", token.expose_secret()).into())
        }
        JmapAuthConfig::Basic { username, password } => {
            let creds = format!("{}:{}", username, password.get()?.expose_secret());
            let encoded = BASE64_STANDARD.encode(creds.into_bytes());
            Ok(format!("Basic {encoded}").into())
        }
    }
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

#[cfg(feature = "maildir")]
impl MaildirConfig {
    pub fn into_client(self) -> io_maildir::client::MaildirClient {
        let root = io_maildir::path::MaildirPath::new(self.root.to_string_lossy().into_owned());
        io_maildir::client::MaildirClient::new(root)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct M2dirConfig {
    /// Filesystem root holding the m2store (a directory carrying a
    /// `.m2store` marker). Each child mailbox is an m2dir with a
    /// `.m2dir` marker.
    pub root: PathBuf,
}

#[cfg(feature = "m2dir")]
impl M2dirConfig {
    pub fn into_client(self) -> io_m2dir::client::M2dirClient {
        let root = io_m2dir::path::M2dirPath::new(self.root.to_string_lossy().into_owned());
        io_m2dir::client::M2dirClient::new(root)
    }
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

impl TryFrom<TlsConfig> for Tls {
    type Error = SecretError;

    fn try_from(config: TlsConfig) -> Result<Self, Self::Error> {
        Ok(Tls {
            provider: config.provider.map(|p| match p {
                TlsProviderConfig::Rustls => TlsProvider::Rustls,
                TlsProviderConfig::NativeTls => TlsProvider::NativeTls,
            }),
            rustls: Rustls {
                crypto: config.rustls.crypto.map(|c| match c {
                    RustlsCryptoConfig::Aws => RustlsCrypto::Aws,
                    RustlsCryptoConfig::Ring => RustlsCrypto::Ring,
                }),
                alpn: Vec::new(),
            },
            cert: config.cert,
        })
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
    pub authcid: String,
    pub passwd: Secret,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslOauthbearerConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub host: String,
    pub port: u16,
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

impl TryFrom<SaslConfig> for Sasl {
    type Error = anyhow::Error;

    fn try_from(config: SaslConfig) -> Result<Self> {
        Ok(match config {
            SaslConfig::Anonymous(c) => Sasl::Anonymous(SaslAnonymous { message: c.message }),
            SaslConfig::Login(c) => Sasl::Login(SaslLogin {
                username: c.username,
                password: c.password.get()?,
            }),
            SaslConfig::Plain(c) => Sasl::Plain(SaslPlain {
                authzid: c.authzid,
                authcid: c.authcid,
                passwd: c.passwd.get()?,
            }),
            SaslConfig::Oauthbearer(c) => Sasl::Oauthbearer(SaslOauthbearer {
                username: c.username,
                host: c.host,
                port: c.port,
                token: c.token.get()?,
            }),
            SaslConfig::Xoauth2(c) => Sasl::Xoauth2(SaslXoauth2 {
                username: c.username,
                token: c.token.get()?,
            }),
            SaslConfig::ScramSha256(c) => Sasl::ScramSha256(SaslScramSha256 {
                username: c.username,
                password: c.password.get()?,
            }),
        })
    }
}
