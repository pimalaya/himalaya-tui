//! TOML configuration model loaded from the same file used by the
//! [`himalaya`] CLI. Each per-backend block deserializes here; the live
//! clients that consume it live in the per-protocol modules
//! (`crate::imap`, `crate::jmap`, …).
//!
//! `Config::project_name()` returns `"himalaya"` (not the crate name)
//! so the default XDG path resolves to `himalaya/config.toml`, allowing
//! the same file to back both binaries.
//!
//! [`himalaya`]: https://github.com/pimalaya/himalaya

use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
#[cfg(feature = "imap")]
use anyhow::anyhow;
#[cfg(feature = "imap")]
use io_imap::types::{
    IntoStatic,
    core::{IString, NString},
};
use pimalaya_config::{
    secret::{Secret, SecretError},
    toml::{TomlConfig, shell_expanded_string},
};
#[cfg(any(feature = "imap", feature = "smtp"))]
use pimalaya_stream::sasl::{
    Sasl, SaslAnonymous, SaslLogin, SaslOauthbearer, SaslPlain, SaslScramSha256, SaslXoauth2,
};
use pimalaya_stream::tls::{Rustls, RustlsCrypto, Tls, TlsProvider};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "imap", feature = "smtp", feature = "jmap"))]
use url::Url;

use crate::tui::{
    model::Keybinds,
    theme::{self, Theme},
};

pub const DEFAULT_PALETTE_KEY: char = ':';

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
    /// Key that opens the command palette;
    /// [`DEFAULT_PALETTE_KEY`] when omitted.
    pub palette_key: Option<char>,
    /// Color theme: pick a preset (`dracula`, `one-dark`, …) and/or
    /// override individual fields. Resolved into a [`Theme`] at
    /// startup.
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
    /// `src/tui/theme/`.
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
/// preset by dropping a new file under `src/tui/theme/`, registering
/// it in `src/tui/theme/mod.rs`, and adding a variant + match arm here.
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
    /// RFC 2971 `ID` extension quirks. Some providers (notably
    /// mail.qq.com, fastmail) require an `ID` exchange straight after
    /// authentication; set `id.auto = true` to opt in.
    #[serde(default)]
    pub id: ImapIdConfig,
}

/// Per-account `imap.id.*` quirks.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ImapIdConfig {
    /// When `true`, the auth coroutine chains an `ID` round-trip
    /// after the tagged auth response. Default `false` skips ID
    /// entirely.
    #[serde(default)]
    pub auto: bool,

    /// Parameters sent with the auto-ID command. Empty (default)
    /// sends `ID NIL`. For each entry: `true` substitutes
    /// himalaya-tui's canned value for the well-known keys (`name`,
    /// `version`, `vendor`, `support-url`) or `NIL` for unknown keys;
    /// `false` always sends `NIL`. Keys absent from this map are not
    /// transmitted.
    #[serde(default)]
    pub fields: HashMap<String, bool>,
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

/// Resolves an [`ImapIdConfig`] into the wire-level parameter list
/// passed to the io-imap auth coroutines.
///
/// [`None`] when `auto = false`; otherwise a vec where each entry
/// maps the user-supplied key to either himalaya-tui's canned value
/// (when the user set `true` and the key is well-known) or `NIL`.
/// Unknown keys with `true` log a warning and fall back to `NIL`.
#[cfg(feature = "imap")]
pub fn resolve_auto_id_params(
    config: &ImapIdConfig,
) -> Result<Option<Vec<(IString<'static>, NString<'static>)>>> {
    if !config.auto {
        return Ok(None);
    }

    let mut params = Vec::with_capacity(config.fields.len());
    for (key, &use_canned) in &config.fields {
        let ikey = IString::try_from(key.clone())
            .map_err(|err| anyhow!("Invalid IMAP ID parameter key `{key}`: {err}"))?
            .into_static();

        let nval = if use_canned {
            match canned_imap_id_value(key) {
                Some(value) => NString::try_from(value)
                    .map_err(|err| {
                        anyhow!("Invalid canned IMAP ID value `{value}` for `{key}`: {err}")
                    })?
                    .into_static(),
                None => {
                    log::warn!("imap.id.fields.{key} = true: no canned value defined, sending NIL");
                    NString::NIL
                }
            }
        } else {
            NString::NIL
        };

        params.push((ikey, nval));
    }
    Ok(Some(params))
}

#[cfg(feature = "imap")]
fn canned_imap_id_value(key: &str) -> Option<&'static str> {
    match key {
        "name" => Some(env!("CARGO_PKG_NAME")),
        "version" => Some(env!("CARGO_PKG_VERSION")),
        "vendor" => Some("Pimalaya"),
        "support-url" => Some("https://github.com/pimalaya/himalaya-tui"),
        _ => None,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct M2dirConfig {
    /// Filesystem root holding the m2store (a directory carrying a
    /// `.m2store` marker). Each child mailbox is an m2dir with a
    /// `.m2dir` marker.
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

#[cfg(any(feature = "imap", feature = "smtp"))]
impl SaslConfig {
    /// Resolves the SASL config into a runtime [`Sasl`]. `host` and
    /// `port` come from the live server URL; they are only used by
    /// OAUTHBEARER (echoed in the GS2 header) and ignored by every
    /// other mechanism.
    pub fn try_into_sasl(self, host: impl ToString, port: u16) -> Result<Sasl> {
        Ok(match self {
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
                host: host.to_string(),
                port,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(body: &str) -> Result<Config, toml::de::Error> {
        toml::from_str(&format!("{body}\n[accounts]\n"))
    }

    #[test]
    fn palette_key_parses_single_character() {
        let config = parse(r#"palette-key = ";""#).unwrap();
        assert_eq!(config.palette_key, Some(';'));
    }

    #[test]
    fn palette_key_defaults_to_colon_when_omitted() {
        let config = parse("").unwrap();
        assert_eq!(config.palette_key, None);
        assert_eq!(DEFAULT_PALETTE_KEY, ':');
    }

    #[test]
    fn palette_key_rejects_multi_character_values() {
        let parsed = parse(r#"palette-key = "abc""#);
        assert!(
            parsed.is_err(),
            "multi-character palette-key must not parse"
        );
    }

    #[test]
    fn palette_key_coexists_with_cli_only_fields() {
        let config = parse("palette-key = \":\"\n\n[table]\nmax-width = 10\n").unwrap();
        assert_eq!(config.palette_key, Some(':'));
    }
}
