//! # Configuration
//!
//! The keys the TUI adds to himalaya's configuration file: the composer's
//! keybindings, the contact command and the theme. Everything else about
//! an account (its backends, identity, signature, mailboxes) is
//! himalaya's, read through himalaya's own types, so the TUI models no
//! backend block and one file reads the same in both binaries.
//!
//! `Config::project_name()` answers "himalaya", so the default paths are
//! the CLI's, and unknown keys are ignored: they are himalaya's.

use std::collections::HashMap;

use pimalaya_config::{command::CommandConfig, toml::TomlConfig};
use ratatui::style::{Color, Modifier, Style};
use serde::Deserialize;

use crate::{
    contact::ContactCompleteKey,
    tui::{
        model::Keybinds,
        theme::{self, Theme},
    },
};

/// The keys the TUI reads from the whole TOML document.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    /// Composer keybinding flavor, Vim when omitted and overridden by
    /// the `--keybinds` flag.
    pub keybinds: Option<Keybinds>,
    /// Fallback for [`AccountConfig::contact_command`].
    pub contact_command: Option<CommandConfig>,
    /// Key completing a recipient in the composer, `tab` when omitted.
    pub contact_complete_key: Option<ContactCompleteKey>,
    /// Color theme, resolved into a [`Theme`] at startup.
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default)]
    pub accounts: HashMap<String, AccountConfig>,
}

impl TomlConfig for Config {
    type Account = AccountConfig;

    /// Answers "himalaya" rather than the crate name, so the default
    /// paths are the very files the CLI reads.
    fn project_name() -> &'static str {
        "himalaya"
    }

    fn sample_url() -> &'static str {
        "https://github.com/pimalaya/himalaya/blob/master/config.sample.toml"
    }

    fn accounts(&self) -> impl Iterator<Item = (&str, bool)> {
        self.accounts
            .iter()
            .map(|(name, account)| (name.as_str(), account.default))
    }

    fn take_named_account(&mut self, name: &str) -> Option<(String, Self::Account)> {
        self.accounts.remove_entry(name)
    }
}

/// The keys the TUI reads from one `[accounts.<name>]` block.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AccountConfig {
    /// Whether this account opens when `-a` names none.
    #[serde(default)]
    pub default: bool,
    /// Command answering the contacts matching a recipient fragment,
    /// winning over the global one.
    pub contact_command: Option<CommandConfig>,
}

/// User-supplied theme: a preset, individual overrides, or both.
///
/// Each override is merged onto the preset with [`Style::patch`], so
/// changing one attribute inherits the rest of the preset.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ThemeConfig {
    /// Preset name, one variant per file under src/tui/theme/.
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
    pub envelope_marked: Option<StyleConfig>,
    pub message_body: Option<StyleConfig>,
    pub compose_text: Option<StyleConfig>,
    pub compose_cursor: Option<StyleConfig>,
    pub compose_selection: Option<StyleConfig>,
}

/// The presets shipped with the binary.
///
/// One is added by dropping a file under src/tui/theme/, declaring it
/// in src/tui/theme.rs, then adding a variant and a match arm here.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PresetConfig {
    Default,
    DraculaDark,
    OneLight,
    TokyoNight,
}

impl PresetConfig {
    /// The theme this preset resolves to.
    pub const fn theme(self) -> Theme {
        match self {
            PresetConfig::Default => theme::default::THEME,
            PresetConfig::DraculaDark => theme::dracula_dark::THEME,
            PresetConfig::OneLight => theme::one_light::THEME,
            PresetConfig::TokyoNight => theme::tokyo_night::THEME,
        }
    }
}

/// Config-side mirror of ratatui's [`Style`], `mod` carrying a list of
/// [`ModifierConfig`] variants.
///
/// ```toml
/// [theme.cursor]
/// fg = "magenta"
/// bg = "#222"
/// mod = ["bold", "italic"]
/// ```
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StyleConfig {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    #[serde(default)]
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

/// Kebab-case mirror of ratatui's [`Modifier`], one variant per flag.
#[derive(Clone, Copy, Debug, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::Config;

    /// The TUI's keys and himalaya's share one document, each reader
    /// ignoring what the other owns.
    #[test]
    fn the_tui_keys_read_beside_himalaya_ones() {
        let toml = r#"
            keybinds = "emacs"
            contact-command = "khard email --parsable %s"
            envelope.list.page-size = 50

            [theme]
            preset = "tokyo-night"

            [accounts.example]
            default = true
            email = "alice@example.org"
            contact-command = ["abook", "--mutt-query", "%s"]
            imap.server = "example.com"
        "#;

        let config: Config = toml::from_str(toml).unwrap();
        let account = config.accounts.get("example").unwrap();

        assert!(config.keybinds.is_some());
        assert!(config.contact_command.is_some());
        assert!(account.default);
        assert!(account.contact_command.is_some());

        let himalaya: himalaya::config::Config = toml::from_str(toml).unwrap();
        assert!(himalaya.accounts.contains_key("example"));
    }

    /// The shipped sample is the reference the README points at, so it
    /// loads in the TUI and in himalaya alike.
    #[test]
    fn shipped_sample_config_loads() {
        let sample = include_str!("../config.sample.toml");

        let config: Config = toml::from_str(sample).expect("the TUI reads the sample");
        assert!(config.accounts.get("example").unwrap().default);

        let himalaya: Result<himalaya::config::Config, _> = toml::from_str(sample);
        assert!(himalaya.is_ok(), "himalaya reads the sample: {himalaya:?}");
    }
}
