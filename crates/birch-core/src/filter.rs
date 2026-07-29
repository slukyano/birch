//! The view filter (task 027): glob patterns that narrow what the tree shows
//! and what a picker may confirm.
//!
//! The corpus rule mirrors search (ADR 0013): a pattern **without** `/` is
//! matched against the entry's simple **name**, one **containing** `/` against
//! its root-relative **path**. So `*.md` is a filename rule and `src/**/*.rs`
//! is a path rule, with no extra flag to explain the difference.
//!
//! Only files are judged. A directory is never dimmed by a filter — file-shaped
//! patterns match no directory at all, and judging directories by them would
//! make the tree unnavigable — but it is *pickable* only when it matches
//! (ADR 0023).

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

/// How non-matching files are presented.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FilterMode {
    /// Shown, dimmed, and inert — the surrounding context stays readable.
    #[default]
    Skip,
    /// Omitted, along with directories known to hold nothing that matches.
    Hide,
}

impl std::fmt::Display for FilterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            FilterMode::Skip => "skip",
            FilterMode::Hide => "hide",
        })
    }
}

impl std::str::FromStr for FilterMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "skip" => Ok(FilterMode::Skip),
            "hide" => Ok(FilterMode::Hide),
            other => Err(format!(
                "unknown filter mode `{other}` (expected hide|skip)"
            )),
        }
    }
}

/// A compiled set of glob patterns. An entry matches when it matches **any**
/// of them.
#[derive(Debug)]
pub struct Filter {
    names: GlobSet,
    paths: GlobSet,
    mode: FilterMode,
}

impl Filter {
    /// Compiles the patterns once, at startup. `None` when no pattern was
    /// given — the absence of a filter, not an empty one that matches nothing.
    /// An invalid pattern is an error naming the pattern, so it can be reported
    /// before the terminal is taken over.
    pub fn parse(patterns: &[String], mode: FilterMode) -> Result<Option<Self>, String> {
        if patterns.is_empty() {
            return Ok(None);
        }
        let mut names = GlobSetBuilder::new();
        let mut paths = GlobSetBuilder::new();
        for pattern in patterns {
            // `literal_separator` keeps `*` inside one path component, so
            // `src/*.rs` does not reach into `src/deep/x.rs`; `**` still does.
            let glob = GlobBuilder::new(pattern)
                .literal_separator(true)
                .build()
                .map_err(|e| format!("bad filter pattern `{pattern}`: {e}"))?;
            if pattern.contains('/') {
                paths.add(glob);
            } else {
                names.add(glob);
            }
        }
        let build = |b: GlobSetBuilder| b.build().map_err(|e| e.to_string());
        Ok(Some(Self {
            names: build(names)?,
            paths: build(paths)?,
            mode,
        }))
    }

    pub fn mode(&self) -> FilterMode {
        self.mode
    }

    /// Whether a file matches: its name against the name patterns, its
    /// root-relative path against the path patterns.
    pub fn matches(&self, rel: &str, name: &str) -> bool {
        self.names.is_match(name) || self.paths.is_match(rel)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filter(patterns: &[&str]) -> Filter {
        let owned: Vec<String> = patterns.iter().map(|p| (*p).to_string()).collect();
        Filter::parse(&owned, FilterMode::Skip)
            .expect("patterns compile")
            .expect("a filter exists")
    }

    #[test]
    fn no_patterns_means_no_filter() {
        assert!(Filter::parse(&[], FilterMode::Skip).unwrap().is_none());
    }

    #[test]
    fn a_bad_pattern_names_itself() {
        let bad = vec!["[".to_string()];
        let error = Filter::parse(&bad, FilterMode::Skip).unwrap_err();
        assert!(
            error.contains('['),
            "the message quotes the pattern: {error}"
        );
    }

    #[test]
    fn plain_patterns_match_the_name_at_any_depth() {
        let f = filter(&["*.md"]);
        assert!(f.matches("README.md", "README.md"));
        assert!(f.matches("docs/deep/guide.md", "guide.md"));
        assert!(!f.matches("src/main.rs", "main.rs"));
    }

    #[test]
    fn patterns_with_a_slash_match_the_relative_path() {
        let f = filter(&["src/*.rs"]);
        assert!(f.matches("src/main.rs", "main.rs"));
        // `*` stays inside one component, so a deeper file does not match.
        assert!(!f.matches("src/cli/args.rs", "args.rs"));
        // And the same name outside src/ does not match either.
        assert!(!f.matches("tests/main.rs", "main.rs"));

        let deep = filter(&["src/**/*.rs"]);
        assert!(deep.matches("src/cli/args.rs", "args.rs"));
    }

    #[test]
    fn any_pattern_matching_is_enough_and_braces_expand() {
        let f = filter(&["*.md", "*.txt"]);
        assert!(f.matches("a.md", "a.md"));
        assert!(f.matches("b.txt", "b.txt"));
        assert!(!f.matches("c.rs", "c.rs"));

        // Brace expansion comes free with globset, so one flag can carry a set.
        let braces = filter(&["*.{md,txt}"]);
        assert!(braces.matches("a.md", "a.md"));
        assert!(braces.matches("b.txt", "b.txt"));
        assert!(!braces.matches("c.rs", "c.rs"));
    }

    #[test]
    fn modes_parse_and_print() {
        assert_eq!("hide".parse(), Ok(FilterMode::Hide));
        assert_eq!("skip".parse(), Ok(FilterMode::Skip));
        assert_eq!(FilterMode::default(), FilterMode::Skip);
        assert_eq!(FilterMode::Hide.to_string(), "hide");
        assert!("nonsense".parse::<FilterMode>().is_err());
    }
}
