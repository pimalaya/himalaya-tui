//! # Configuration
//!
//! The keys the TUI adds to himalaya's configuration file, under a `tui`
//! table at the top level and in each account: the composer's
//! keybindings, the contact command and the theme. Everything else about
//! an account (its backends, identity, signature, mailboxes) is
//! himalaya's, read through himalaya's own types, so the TUI models no
//! backend block and one file reads the same in both binaries.
//!
//! `Config::project_name()` answers "himalaya", so the default paths are
//! the CLI's. Keys outside the `tui` tables are ignored, being himalaya's,
//! and the `tui` tables refuse a key they do not know, as himalaya refuses
//! one outside them.

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
    /// The TUI's own top-level keys.
    #[serde(default)]
    pub tui: TuiConfig,
    #[serde(default)]
    pub accounts: HashMap<String, AccountConfig>,
}

/// The top-level `[tui]` table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TuiConfig {
    /// Composer keybinding flavor, Vim when omitted and overridden by
    /// the `--keybinds` flag.
    pub keybinds: Option<Keybinds>,
    /// Fallback for [`AccountTuiConfig::contact_command`].
    pub contact_command: Option<CommandConfig>,
    /// Key completing a recipient in the composer, `tab` when omitted.
    pub contact_complete_key: Option<ContactCompleteKey>,
    /// Color theme, resolved into a [`Theme`] at startup.
    #[serde(default)]
    pub theme: ThemeConfig,
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
    /// The TUI's own keys of this account.
    #[serde(default)]
    pub tui: AccountTuiConfig,
}

/// One `[accounts.<name>.tui]` table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AccountTuiConfig {
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
/// [tui.theme.cursor]
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
    use std::{fs, path::PathBuf};

    use himalaya::cli::resolve_account;
    use pimalaya_config::{command::CommandConfig, toml::TomlConfig};
    use ratatui::style::Color;
    use tempfile::TempDir;

    use crate::{
        config::Config,
        contact::ContactCompleteKey,
        tui::{
            model::Keybinds,
            theme::{self, Theme},
        },
    };

    /// Writes `toml` to a configuration file of a fresh directory.
    fn write(toml: &str) -> (TempDir, Vec<PathBuf>) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, toml).unwrap();
        (dir, vec![path])
    }

    /// The TUI reads its keys from the `tui` tables, and himalaya loads
    /// the same file, leaving them to the TUI.
    #[test]
    fn the_tui_tables_read_beside_himalaya_keys() {
        let (_dir, paths) = write(
            r#"
            envelope.list.page-size = 50

            [tui]
            keybinds = "emacs"
            contact-command = "khard email --parsable %s"
            contact-complete-key = "ctrl-space"

            [tui.theme]
            preset = "tokyo-night"

            [tui.theme.cursor]
            fg = "magenta"

            [accounts.work]
            default = true
            email = "alice@example.org"
            maildir.root = "~/Mail"

            [accounts.work.tui]
            contact-command = ["abook", "--mutt-query", "%s"]
            "#,
        );

        let mut config = Config::load(&paths).unwrap();
        let account = config.accounts.remove("work").unwrap();
        let theme = Theme::resolve(&config.tui.theme);
        let key: ContactCompleteKey = "ctrl-space".parse().unwrap();
        let khard = CommandConfig::Shell("khard email --parsable %s".into());
        let abook = CommandConfig::Argv {
            program: "abook".into(),
            args: vec!["--mutt-query".into(), "%s".into()],
        };

        assert_eq!(config.tui.keybinds, Some(Keybinds::Emacs));
        assert_eq!(config.tui.contact_command, Some(khard));
        assert_eq!(config.tui.contact_complete_key, Some(key));
        assert_eq!(theme.header, theme::tokyo_night::THEME.header);
        assert_eq!(theme.cursor.fg, Some(Color::Magenta));
        assert!(account.default);
        assert_eq!(account.tui.contact_command, Some(abook));

        let (_, name, account) = resolve_account(&paths, None).unwrap();
        assert_eq!(name, "work");
        assert_eq!(account.email.as_deref(), Some("alice@example.org"));
    }

    /// A key the `tui` tables do not know fails the load, naming it.
    #[test]
    fn an_unknown_tui_key_is_refused() {
        let tables = [
            "[tui]\nkeybind = \"emacs\"\n",
            "[tui.theme]\npreset = \"default\"\nheadr.fg = \"blue\"\n",
            "[accounts.work.tui]\ncontact-cmd = \"khard\"\n",
        ];

        for (toml, key) in tables.into_iter().zip(["keybind", "headr", "contact-cmd"]) {
            let (_dir, paths) = write(toml);
            let err = format!("{:#}", Config::load(&paths).unwrap_err());
            assert!(err.contains(&format!("unknown field `{key}`")), "{err}");
        }
    }

    /// The TUI keys written outside the `tui` tables are not read, and
    /// himalaya refuses them.
    #[test]
    fn the_tui_keys_outside_the_tui_tables_are_not_read() {
        let (_root_dir, root_paths) = write(
            r#"
            keybinds = "emacs"
            contact-command = "khard email --parsable %s"
            contact-complete-key = "ctrl-space"

            [theme]
            preset = "tokyo-night"

            [accounts.work]
            default = true
            "#,
        );

        let config = Config::load(&root_paths).unwrap();

        assert_eq!(config.tui.keybinds, None);
        assert_eq!(config.tui.contact_command, None);
        assert_eq!(config.tui.contact_complete_key, None);
        assert!(config.tui.theme.preset.is_none());

        let err = format!("{:#}", resolve_account(&root_paths, None).unwrap_err());
        let keys = [
            "keybinds",
            "contact-command",
            "contact-complete-key",
            "theme",
        ];
        let named = keys.map(|key| format!("unknown field `{key}`"));
        assert!(named.iter().any(|key| err.contains(key)), "{err}");

        let (_account_dir, account_paths) = write(
            r#"
            [accounts.work]
            default = true
            contact-command = ["abook", "--mutt-query", "%s"]
            "#,
        );

        let mut config = Config::load(&account_paths).unwrap();
        let account = config.accounts.remove("work").unwrap();

        assert_eq!(account.tui.contact_command, None);

        let err = format!("{:#}", resolve_account(&account_paths, None).unwrap_err());
        assert!(err.contains("unknown field `contact-command`"), "{err}");
    }

    /// The shipped sample is the reference the README points at, so it
    /// loads in the TUI and in himalaya alike, as written and uncommented.
    #[test]
    fn shipped_sample_config_loads() {
        let sample = include_str!("../config.sample.toml");

        let config: Config = toml::from_str(sample).expect("the TUI reads the sample");
        assert!(config.accounts.get("example").unwrap().default);

        let himalaya: Result<himalaya::config::Config, _> = toml::from_str(sample);
        assert!(himalaya.is_ok(), "himalaya reads the sample: {himalaya:?}");

        let uncommented: String = sample
            .lines()
            .map(|line| match line.strip_prefix('#') {
                Some(code) if code.starts_with(|c: char| c == '[' || c.is_ascii_lowercase()) => {
                    code
                }
                _ => line,
            })
            .flat_map(|line| [line, "\n"])
            .collect();

        let mut config: Config = toml::from_str(&uncommented).expect("the TUI reads it all");
        let account = config.accounts.remove("example").unwrap();

        assert_eq!(config.tui.keybinds, Some(Keybinds::Vim));
        assert!(config.tui.contact_command.is_some());
        assert!(config.tui.theme.preset.is_some());
        assert!(config.tui.theme.compose_selection.is_some());
        assert!(account.tui.contact_command.is_some());

        let himalaya: Result<himalaya::config::Config, _> = toml::from_str(&uncommented);
        assert!(himalaya.is_ok(), "himalaya reads it all: {himalaya:?}");
    }
}
