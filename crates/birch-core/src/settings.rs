//! Runtime settings shared across the app. Plain data; flags set initial
//! values, `birch-ctl set` will change them at runtime later.

/// Names hidden by default as pure noise (shown with `--show-noise`).
pub const NOISE: &[&str] = &[".git", ".DS_Store", "Thumbs.db"];

#[derive(Clone, Debug)]
pub struct Settings {
    pub icons: bool,
    pub files_first: bool,
    pub show_hidden: bool,
    pub show_noise: bool,
    pub mouse: bool,
    pub git: bool,
    /// Gitignored entries: shown dimmed when true, hidden when false.
    pub show_ignored: bool,
    pub compact: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            icons: true,
            files_first: false,
            show_hidden: true,
            show_noise: false,
            mouse: true,
            git: true,
            show_ignored: true,
            compact: true,
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
}
