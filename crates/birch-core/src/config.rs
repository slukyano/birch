//! Personal defaults from `~/.config/birch/birch.toml` (ADR 0022). Pure data —
//! no ratatui. Parsing is **tolerant**: unknown keys are ignored, and a
//! missing or malformed file degrades to built-in defaults with a warning,
//! never blocking launch. Config is the default source in the precedence
//! chain `Settings::default()` → config → CLI flags → `ctl set`.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::settings::{Settings, ThemeId, clamp_scroll_lines};

/// A partial set of personal defaults parsed from the TOML config. Every field
/// is optional — a missing key means "no opinion, use the built-in default".
/// Unknown keys are ignored (no `deny_unknown_fields`) so newer configs stay
/// readable by older birch.
#[derive(Deserialize, Default, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub theme: Option<ThemeId>,
    pub icons: Option<bool>,
    pub git: Option<bool>,
    pub hidden: Option<bool>,
    pub ignored: Option<bool>,
    pub noise: Option<bool>,
    pub compact: Option<bool>,
    pub mouse: Option<bool>,
    pub open_cmd: Option<String>,
    pub scroll_lines: Option<i64>,
    pub scrollbar: Option<bool>,
}

impl Config {
    /// The resolved config path: `$XDG_CONFIG_HOME/birch/birch.toml` if
    /// `XDG_CONFIG_HOME` is set (to an absolute path), else
    /// `~/.config/birch/birch.toml`. Falls back to a relative
    /// `.config/birch/birch.toml` only if `$HOME` is unset — never panics.
    pub fn path() -> PathBuf {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
            .unwrap_or_else(|| PathBuf::from(".config"));
        base.join("birch").join("birch.toml")
    }

    /// Loads the config, tolerantly. Reads `explicit` if given, else the
    /// resolved default path. A missing file yields `Config::default()` with no
    /// warning; a malformed or unreadable file yields `Config::default()` and a
    /// warning string for the caller to print to stderr before the TUI starts.
    /// Never panics, never fails launch.
    pub fn load(explicit: Option<&Path>) -> (Config, Option<String>) {
        let path = explicit.map(Path::to_path_buf).unwrap_or_else(Self::path);
        match std::fs::read_to_string(&path) {
            Ok(text) => match toml::from_str::<Config>(&text) {
                Ok(config) => (config, None),
                Err(e) => (
                    Config::default(),
                    Some(format!(
                        "birch: ignoring malformed config {}: {e}",
                        path.display()
                    )),
                ),
            },
            // A missing file is the normal, silent case — no config, no warning.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (Config::default(), None),
            // Anything else (permissions, etc.) is a readable-but-not case: warn.
            Err(e) => (
                Config::default(),
                Some(format!(
                    "birch: ignoring unreadable config {}: {e}",
                    path.display()
                )),
            ),
        }
    }

    /// Overrides only the present (`Some`) fields on `s`, leaving the rest at
    /// whatever they were (typically `Settings::default()`). Does not touch the
    /// open command — that is `open_cmd`, resolved in `main`.
    pub fn apply_to(&self, s: &mut Settings) {
        if let Some(theme) = self.theme {
            s.theme = theme;
        }
        if let Some(icons) = self.icons {
            s.icons = icons;
        }
        if let Some(git) = self.git {
            s.git = git;
        }
        if let Some(hidden) = self.hidden {
            s.show_hidden = hidden;
        }
        if let Some(ignored) = self.ignored {
            s.show_ignored = ignored;
        }
        if let Some(noise) = self.noise {
            s.show_noise = noise;
        }
        if let Some(compact) = self.compact {
            s.compact = compact;
        }
        if let Some(mouse) = self.mouse {
            s.mouse = mouse;
        }
        if let Some(scrollbar) = self.scrollbar {
            s.scrollbar = scrollbar;
        }
        // Out of range degrades rather than failing: a config file must never
        // block launch (ADR 0022).
        if let Some(scroll_lines) = self.scroll_lines {
            s.scroll_lines = clamp_scroll_lines(scroll_lines);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("birch-config-{}-{}.toml", tag, std::process::id()))
    }

    #[test]
    fn parses_a_full_config() {
        let toml = r#"
            theme = "jetbrains"
            icons = false
            git = false
            hidden = false
            ignored = false
            noise = true
            compact = false
            mouse = false
            open-cmd = "code {}"
            scroll-lines = 5
            scrollbar = false
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.theme, Some(ThemeId::Jetbrains));
        assert_eq!(config.icons, Some(false));
        assert_eq!(config.git, Some(false));
        assert_eq!(config.hidden, Some(false));
        assert_eq!(config.ignored, Some(false));
        assert_eq!(config.noise, Some(true));
        assert_eq!(config.compact, Some(false));
        assert_eq!(config.mouse, Some(false));
        assert_eq!(config.open_cmd.as_deref(), Some("code {}"));
        assert_eq!(config.scroll_lines, Some(5));
        assert_eq!(config.scrollbar, Some(false));
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let toml = r#"
            theme = "retro"
            future-setting = "whatever"
            [nested]
            also = true
        "#;
        let config: Config = toml::from_str(toml).unwrap();
        assert_eq!(config.theme, Some(ThemeId::Retro));
        assert_eq!(config.icons, None);
    }

    #[test]
    fn empty_config_is_all_none() {
        let config: Config = toml::from_str("").unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn missing_file_yields_default_without_warning() {
        let path = temp_path("missing");
        let _ = std::fs::remove_file(&path);
        let (config, warning) = Config::load(Some(&path));
        assert_eq!(config, Config::default());
        assert!(warning.is_none());
    }

    #[test]
    fn malformed_file_yields_default_with_warning() {
        let path = temp_path("malformed");
        std::fs::write(&path, "this is = = not toml").unwrap();
        let (config, warning) = Config::load(Some(&path));
        assert_eq!(config, Config::default());
        assert!(warning.is_some());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn load_reads_a_valid_explicit_file() {
        let path = temp_path("valid");
        std::fs::write(&path, "theme = \"xcode\"\nicons = false\n").unwrap();
        let (config, warning) = Config::load(Some(&path));
        assert!(warning.is_none());
        assert_eq!(config.theme, Some(ThemeId::Xcode));
        assert_eq!(config.icons, Some(false));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn apply_to_overrides_only_present_fields() {
        let config = Config {
            theme: Some(ThemeId::Vscode),
            icons: Some(false),
            git: None,
            hidden: Some(false),
            ignored: Some(false),
            noise: Some(true),
            compact: Some(false),
            mouse: Some(false),
            open_cmd: Some("vim {}".into()),
            scroll_lines: None,
            scrollbar: None,
        };
        let mut settings = Settings::default();
        config.apply_to(&mut settings);
        assert_eq!(settings.theme, ThemeId::Vscode);
        assert!(!settings.icons);
        // `git` was None — keeps the default.
        assert!(settings.git);
        assert!(!settings.show_hidden);
        assert!(!settings.show_ignored);
        assert!(settings.show_noise);
        assert!(!settings.compact);
        assert!(!settings.mouse);
    }

    #[test]
    fn apply_to_empty_config_leaves_defaults() {
        let mut settings = Settings::default();
        Config::default().apply_to(&mut settings);
        let default = Settings::default();
        assert_eq!(settings.icons, default.icons);
        assert_eq!(settings.show_hidden, default.show_hidden);
        assert_eq!(settings.show_noise, default.show_noise);
        assert_eq!(settings.mouse, default.mouse);
        assert_eq!(settings.git, default.git);
        assert_eq!(settings.show_ignored, default.show_ignored);
        assert_eq!(settings.compact, default.compact);
        assert_eq!(settings.theme, default.theme);
    }

    #[test]
    fn path_prefers_xdg_config_home() {
        // Reading env is process-global; this test only reads, never mutates,
        // and asserts the shape of the resolved path.
        let path = Config::path();
        assert!(path.ends_with("birch/birch.toml"));
    }

    #[test]
    fn scroll_lines_out_of_range_is_clamped_not_rejected() {
        // A config file must never block launch (ADR 0022), so an impossible
        // value degrades into the accepted range.
        let mut s = Settings::default();
        Config {
            scroll_lines: Some(250),
            ..Config::default()
        }
        .apply_to(&mut s);
        assert_eq!(s.scroll_lines, crate::settings::SCROLL_LINES_MAX);

        let mut s = Settings::default();
        Config {
            scroll_lines: Some(0),
            ..Config::default()
        }
        .apply_to(&mut s);
        assert_eq!(s.scroll_lines, crate::settings::SCROLL_LINES_MIN);
    }

    #[test]
    fn a_scroll_lines_too_large_for_a_byte_still_clamps() {
        // The value must degrade on its own, not take the rest of the file
        // with it: a parse failure discards every other key too (ADR 0022).
        let toml = r#"
            theme = "nord"
            scroll-lines = 300
        "#;
        let config: Config = toml::from_str(toml).expect("still parses");
        let mut s = Settings::default();
        config.apply_to(&mut s);
        assert_eq!(s.scroll_lines, crate::settings::SCROLL_LINES_MAX);
        assert_eq!(s.theme, ThemeId::Nord, "the rest of the file survived");

        let toml = "scroll-lines = -7";
        let config: Config = toml::from_str(toml).expect("still parses");
        let mut s = Settings::default();
        config.apply_to(&mut s);
        assert_eq!(s.scroll_lines, crate::settings::SCROLL_LINES_MIN);
    }

    #[test]
    fn scrollbar_applies_from_the_config() {
        let mut s = Settings::default();
        assert!(s.scrollbar, "on by default");
        Config {
            scrollbar: Some(false),
            ..Config::default()
        }
        .apply_to(&mut s);
        assert!(!s.scrollbar);
    }

    #[test]
    fn absent_scroll_lines_keeps_the_default() {
        let mut s = Settings::default();
        Config::default().apply_to(&mut s);
        assert_eq!(s.scroll_lines, crate::settings::SCROLL_LINES_DEFAULT);
    }
}
