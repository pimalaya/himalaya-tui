use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use pimalaya_config::{
    secret::{Secret, SecretError},
    toml::{shell_expanded_string, TomlConfig},
};
use pimalaya_stream::{
    sasl::{
        Sasl, SaslAnonymous, SaslLogin, SaslOauthbearer, SaslPlain, SaslScramSha256, SaslXoauth2,
    },
    tls::{Rustls, RustlsCrypto, Tls, TlsProvider},
};
use serde::Deserialize;
use url::Url;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Config {
    #[serde(alias = "name")]
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    fn project_name() -> &'static str {
        env!("CARGO_PKG_NAME")
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountConfig {
    #[serde(default)]
    pub default: bool,
    pub imap: Option<ImapConfig>,
    pub smtp: Option<SmtpConfig>,
    pub jmap: Option<JmapConfig>,
    #[serde(deserialize_with = "shell_expanded_string")]
    pub email: String,
    pub display_name: Option<String>,
    pub signature: Option<String>,
    pub signature_delim: Option<String>,
    pub downloads_dir: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
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
    use base64::{prelude::BASE64_STANDARD, Engine};
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TlsConfig {
    pub provider: Option<TlsProviderConfig>,
    #[serde(default)]
    pub rustls: RustlsConfig,
    pub cert: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TlsProviderConfig {
    Rustls,
    NativeTls,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RustlsConfig {
    pub crypto: Option<RustlsCryptoConfig>,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslAnonymousConfig {
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslLoginConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub password: Secret,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslPlainConfig {
    pub authzid: Option<String>,
    #[serde(deserialize_with = "shell_expanded_string")]
    pub authcid: String,
    pub passwd: Secret,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslOauthbearerConfig {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub host: String,
    pub port: u16,
    pub token: Secret,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SaslXoauth2Config {
    #[serde(deserialize_with = "shell_expanded_string")]
    pub username: String,
    pub token: Secret,
}

#[derive(Clone, Debug, Deserialize)]
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
