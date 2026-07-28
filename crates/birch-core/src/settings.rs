//! Runtime settings shared across the app. Plain data; flags set initial
//! values, `birch-ctl set` will change them at runtime later.

use serde::{Deserialize, Serialize};

/// Names hidden by default as pure noise (shown with `--show-noise`).
pub const NOISE: &[&str] = &[".git", ".DS_Store", "Thumbs.db"];

/// Which built-in theme selects the render layer's visual vocabulary
/// (ADR 0021). Core only *names* the theme — the ratatui `Theme` definition
/// lives in `birch-tui`, so the crate stays ratatui-free.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeId {
    #[default]
    Birch,
    Vscode,
    Jetbrains,
    Xcode,
    Mocha,
    Tokyonight,
    Gruvbox,
    Nord,
    Rosepine,
    Retro,
    Plain,
}

impl std::fmt::Display for ThemeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ThemeId::Birch => "birch",
            ThemeId::Vscode => "vscode",
            ThemeId::Jetbrains => "jetbrains",
            ThemeId::Xcode => "xcode",
            ThemeId::Mocha => "mocha",
            ThemeId::Tokyonight => "tokyonight",
            ThemeId::Gruvbox => "gruvbox",
            ThemeId::Nord => "nord",
            ThemeId::Rosepine => "rosepine",
            ThemeId::Retro => "retro",
            ThemeId::Plain => "plain",
        })
    }
}

impl std::str::FromStr for ThemeId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "birch" => Ok(ThemeId::Birch),
            "vscode" => Ok(ThemeId::Vscode),
            "jetbrains" => Ok(ThemeId::Jetbrains),
            "xcode" => Ok(ThemeId::Xcode),
            "mocha" => Ok(ThemeId::Mocha),
            "tokyonight" => Ok(ThemeId::Tokyonight),
            "gruvbox" => Ok(ThemeId::Gruvbox),
            "nord" => Ok(ThemeId::Nord),
            "rosepine" => Ok(ThemeId::Rosepine),
            "retro" => Ok(ThemeId::Retro),
            "plain" => Ok(ThemeId::Plain),
            other => Err(format!("unknown theme: {other}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub icons: bool,
    pub show_hidden: bool,
    pub show_noise: bool,
    pub mouse: bool,
    pub git: bool,
    /// Gitignored entries: shown dimmed when true, hidden when false.
    pub show_ignored: bool,
    pub compact: bool,
    /// The active theme (ADR 0021); the render layer resolves it to a `Theme`.
    pub theme: ThemeId,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            icons: true,
            show_hidden: true,
            show_noise: false,
            mouse: true,
            git: true,
            show_ignored: true,
            compact: true,
            theme: ThemeId::default(),
        }
    }
}

pub fn is_noise(name: &str) -> bool {
    NOISE.contains(&name)
}

pub fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_and_hidden() {
        assert!(is_noise(".git"));
        assert!(!is_noise(".gitignore"));
        assert!(is_hidden(".gitignore"));
        assert!(!is_hidden("src"));
    }

    #[test]
    fn theme_id_parses_and_displays_kebab_ids() {
        use std::str::FromStr;
        for id in [
            ThemeId::Birch,
            ThemeId::Vscode,
            ThemeId::Jetbrains,
            ThemeId::Xcode,
            ThemeId::Mocha,
            ThemeId::Tokyonight,
            ThemeId::Gruvbox,
            ThemeId::Nord,
            ThemeId::Rosepine,
            ThemeId::Retro,
            ThemeId::Plain,
        ] {
            assert_eq!(ThemeId::from_str(&id.to_string()), Ok(id));
        }
        assert_eq!(ThemeId::default(), ThemeId::Birch);
        assert!(ThemeId::from_str("nope").is_err());
        // Serde uses the same kebab ids as Display/FromStr.
        assert_eq!(
            serde_json::to_string(&ThemeId::Jetbrains).unwrap(),
            r#""jetbrains""#
        );
    }
}
