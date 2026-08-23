//! Open-command templates: `{}` is the path. Parsing and argv construction
//! live here (pure, testable); execution is the binary's job because it owns
//! the terminal.

use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OpenMode {
    /// Hand the terminal over and wait (terminal editors).
    Terminal,
    /// Fire and forget (GUI dispatchers like `open` / `xdg-open`).
    Detached,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenCmd {
    argv: Vec<String>,
    pub mode: OpenMode,
}

impl OpenCmd {
    /// Parses a user-supplied template (`--open-cmd`). Shell-style word
    /// splitting; no shell is ever invoked.
    pub fn from_template(template: &str) -> Result<Self, String> {
        let argv = shell_words::split(template).map_err(|e| format!("bad open command: {e}"))?;
        if argv.is_empty() {
            return Err("open command is empty".into());
        }
        // Fail loudly on the once-documented placeholder rather than passing
        // it through as a literal argument.
        if argv.iter().any(|arg| arg.contains("{line}")) {
            return Err(
                "open command: {line} is not a supported placeholder (only {} is substituted)"
                    .into(),
            );
        }
        Ok(Self {
            argv,
            mode: OpenMode::Terminal,
        })
    }

    /// The default: `$VISUAL {}` when set, else `$EDITOR {}`, else the
    /// platform opener (VISUAL is the conventional full-screen editor;
    /// EDITOR historically names the line editor).
    pub fn default_cmd() -> Self {
        Self::default_from(
            std::env::var("VISUAL").ok().as_deref(),
            std::env::var("EDITOR").ok().as_deref(),
        )
    }

    fn default_from(visual: Option<&str>, editor: Option<&str>) -> Self {
        for candidate in [visual, editor].into_iter().flatten() {
            if let Ok(mut argv) = shell_words::split(candidate)
                && !argv.is_empty()
            {
                argv.push("{}".into());
                return Self {
                    argv,
                    mode: OpenMode::Terminal,
                };
            }
        }
        Self::platform_opener()
    }

    /// The desktop's own handler — `open` on macOS, `xdg-open` elsewhere —
    /// spawned detached, since it hands off to another application rather than
    /// taking the terminal. It is both the last fallback for `default_cmd` and
    /// the `Open system` menu entry (task 034).
    pub fn platform_opener() -> Self {
        Self {
            argv: vec![Self::platform_program().into(), "{}".into()],
            mode: OpenMode::Detached,
        }
    }

    /// The platform opener's program name on its own. The menu names it rather
    /// than describing it: `open` and `xdg-open` are things a person can run,
    /// and "system" is not.
    pub fn platform_program() -> &'static str {
        if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        }
    }

    /// Shows the path *in* the desktop's file manager instead of opening it —
    /// Finder's "Reveal in Finder". macOS has a flag for exactly this; nothing
    /// portable exists elsewhere, so the containing directory is opened
    /// instead, which is the same answer one step coarser. Pair it with
    /// [`Self::reveal_target`], which decides what to point it at.
    pub fn reveal() -> Self {
        let argv = if cfg!(target_os = "macos") {
            vec!["open".into(), "-R".into(), "{}".into()]
        } else {
            vec![Self::platform_program().into(), "{}".into()]
        };
        Self {
            argv,
            mode: OpenMode::Detached,
        }
    }

    /// What [`Self::reveal`] is pointed at: the path itself where the platform
    /// can select it, and its containing directory where it cannot.
    pub fn reveal_target(path: &Path) -> PathBuf {
        if cfg!(target_os = "macos") {
            path.to_path_buf()
        } else {
            path.parent().unwrap_or(path).to_path_buf()
        }
    }

    /// The program this command runs, without its arguments. What the menu
    /// shows beside `Open`, so the row names what will actually happen rather
    /// than a category — `nvim +{line} {}` truncated is noise, `nvim` is not.
    pub fn program(&self) -> &str {
        self.argv.first().map_or("", |arg| {
            arg.rsplit(std::path::MAIN_SEPARATOR).next().unwrap_or(arg)
        })
    }

    /// Substitutes the template into a concrete argv. If no arg contains
    /// `{}`, the path is appended.
    pub fn build(&self, path: &Path) -> Vec<String> {
        let path_str = path.to_string_lossy();
        let mut argv = Vec::with_capacity(self.argv.len() + 1);
        let mut has_path = false;
        for arg in &self.argv {
            if arg.contains("{}") {
                has_path = true;
                argv.push(arg.replace("{}", &path_str));
            } else {
                argv.push(arg.clone());
            }
        }
        if !has_path {
            argv.push(path_str.into_owned());
        }
        argv
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn reveal_shows_the_path_without_opening_it() {
        let cmd = OpenCmd::reveal();
        assert_eq!(cmd.mode, OpenMode::Detached, "it hands off to another app");
        let path = Path::new("/r/src/main.rs");
        let argv = cmd.build(&OpenCmd::reveal_target(path));
        if cfg!(target_os = "macos") {
            // `-R` selects the file rather than opening it.
            assert_eq!(argv, vec!["open", "-R", "/r/src/main.rs"]);
        } else {
            // Nothing portable selects a file, so the folder holding it opens.
            assert_eq!(argv, vec![OpenCmd::platform_program(), "/r/src"]);
        }
    }

    #[test]
    fn substitutes_path() {
        let cmd = OpenCmd::from_template("code -g {}").unwrap();
        assert_eq!(
            cmd.build(&PathBuf::from("/a/b.rs")),
            ["code", "-g", "/a/b.rs"]
        );
    }

    #[test]
    fn appends_path_when_no_placeholder() {
        let cmd = OpenCmd::from_template("nvim").unwrap();
        assert_eq!(cmd.build(&PathBuf::from("/a/b.rs")), ["nvim", "/a/b.rs"]);
    }

    #[test]
    fn respects_quoting() {
        let cmd = OpenCmd::from_template("myeditor --flag 'a b' {}").unwrap();
        assert_eq!(
            cmd.build(&PathBuf::from("/x")),
            ["myeditor", "--flag", "a b", "/x"]
        );
    }

    #[test]
    fn rejects_empty_template() {
        assert!(OpenCmd::from_template("   ").is_err());
    }

    #[test]
    fn rejects_stale_line_placeholder() {
        assert!(OpenCmd::from_template("nvim +{line} {}").is_err());
    }

    #[test]
    fn default_prefers_visual_then_editor_then_opener() {
        let cmd = OpenCmd::default_from(Some("myvisual -f"), Some("myeditor"));
        assert_eq!(cmd.build(&PathBuf::from("/x")), ["myvisual", "-f", "/x"]);
        assert_eq!(cmd.mode, OpenMode::Terminal);

        let cmd = OpenCmd::default_from(None, Some("myeditor"));
        assert_eq!(cmd.build(&PathBuf::from("/x")), ["myeditor", "/x"]);

        // Blank values fall through to the next candidate.
        let cmd = OpenCmd::default_from(Some("   "), Some("myeditor"));
        assert_eq!(cmd.build(&PathBuf::from("/x")), ["myeditor", "/x"]);

        let cmd = OpenCmd::default_from(None, None);
        assert_eq!(cmd.mode, OpenMode::Detached);
    }
}
