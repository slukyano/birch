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
    /// What the menu calls this command, when the argv[0] basename is the
    /// wrong answer (task 096). A host adapter is a courier, not a program a
    /// person opens files with, so it says what it routes to instead.
    name: Option<String>,
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
            name: None,
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
                    name: None,
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
            name: None,
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
            name: None,
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

    /// What the menu shows beside `Open`: the display name when one was set,
    /// otherwise the program. `nvim +{line} {}` still reads `nvim`; a host
    /// adapter reads the host it routes to rather than its own filename.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or_else(|| self.program())
    }

    /// The display name as set, or `None` when the program shows through.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Sets the display name, cleaned. An empty result clears it, so the
    /// program shows through again rather than the row going blank.
    pub fn set_name(&mut self, raw: &str) {
        let cleaned = sanitize_name(raw);
        self.name = (!cleaned.is_empty()).then_some(cleaned);
    }

    /// Whether this is the desktop handler, ignoring any display name — the
    /// menu suppresses its `Open with the system handler` row when `Open`
    /// already runs it, and renaming a command does not change what it runs.
    pub fn is_platform_opener(&self) -> bool {
        let platform = Self::platform_opener();
        self.argv == platform.argv && self.mode == platform.mode
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

/// The most a display name may occupy in a menu row. `Open with ` plus this
/// already overflows a narrow pane, so the cap costs nothing a caller wanted
/// and removes the question of what a careless value does to the frame.
pub const NAME_LIMIT: usize = 32;

/// Cleans a display name at the boundary it arrives on. One path is a launch
/// flag and the other is the control socket, and two paths with different
/// rules is how one of them ends up wrong — so both take this.
pub fn sanitize_name(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_control())
        .take(NAME_LIMIT)
        .collect::<String>()
        .trim()
        .to_string()
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

    #[test]
    fn the_display_name_replaces_the_program_only_when_set() {
        let mut cmd = OpenCmd::from_template("birch-cmux preview {}").expect("template");
        assert_eq!(cmd.display_name(), "birch-cmux");
        assert_eq!(cmd.name(), None);
        cmd.set_name("cmux");
        assert_eq!(cmd.display_name(), "cmux");
        // The program is untouched: the row is renamed, not the command.
        assert_eq!(cmd.program(), "birch-cmux");
    }

    #[test]
    fn a_name_is_cleaned_and_bounded_where_it_arrives() {
        let mut cmd = OpenCmd::from_template("adapter {}").expect("template");
        cmd.set_name("cm\nux\u{7}");
        assert_eq!(cmd.display_name(), "cmux");
        cmd.set_name(&"x".repeat(200));
        assert_eq!(cmd.display_name().chars().count(), NAME_LIMIT);
    }

    #[test]
    fn an_empty_name_lets_the_program_show_through_again() {
        let mut cmd = OpenCmd::from_template("adapter {}").expect("template");
        cmd.set_name("cmux");
        cmd.set_name("   ");
        assert_eq!(cmd.name(), None);
        assert_eq!(cmd.display_name(), "adapter");
    }

    #[test]
    fn renaming_the_platform_opener_does_not_make_it_something_else() {
        let mut cmd = OpenCmd::platform_opener();
        assert!(cmd.is_platform_opener());
        cmd.set_name("desktop");
        // The menu suppresses its system-handler row when `Open` already runs
        // it, and a display name changes nothing about what runs.
        assert!(cmd.is_platform_opener());
    }
}
