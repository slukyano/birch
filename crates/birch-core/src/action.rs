//! The action layer (ADR 0029): every thing birch can do to a selection,
//! declared once. Hotkeys, the mouse, the context menu, and the socket all
//! route through it, and the menu is *generated* from it rather than authored
//! beside it — which is what keeps menu-specific logic out of the codebase.
//!
//! No ratatui here: an action is a decision about paths, not about painting.

use std::path::PathBuf;

/// One thing birch can do. Adding a variant, its metadata, and its availability
/// rule is the whole of adding a menu entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Open a file, or expand/collapse a directory. In the picker it confirms.
    Open,
    /// Hand the path to the desktop's own handler, detached (task 034).
    OpenSystem,
    /// Create a file under the target, inline.
    NewFile,
    /// Create a directory under the target, inline. `NewFile` reaches the same
    /// place with a trailing slash; this is the discoverable way, since the
    /// menu is where actions are found (ADR 0029).
    NewFolder,
    /// Rename the target inline. An input carrying separators moves it.
    Rename,
    /// Move the selection to the platform trash (ADR 0031).
    Delete,
    /// Copy the selection's file name to the clipboard.
    CopyName,
    /// Copy the selection's path, relative to the birch root.
    CopyRelativePath,
    /// Copy the selection's absolute path.
    CopyAbsolutePath,
    /// Re-root the tree at the selected directory.
    RevealRootHere,
    /// Reverse the most recent mutation (task 093).
    Undo,
    /// Show the path in the desktop's file manager, without opening it.
    Reveal,
}

/// Which form of a path an action copies. Mirrors the socket's `PathForm`, so
/// the menu and `birch ctl get-path` cannot drift apart about what "the
/// relative path" means.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyForm {
    Name,
    Relative,
    Absolute,
}

/// A menu label in two parts. The **verb** is fixed and drawn at full weight;
/// the **subject** is the thing it names — a program — and is drawn dim, so a
/// row reads as an instruction with a parameter rather than one compound
/// phrase where `Open vim` could be a verb nobody has heard of.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Label {
    pub verb: String,
    pub subject: Option<String>,
}

impl Label {
    pub fn plain(verb: &str) -> Self {
        Self {
            verb: verb.to_string(),
            subject: None,
        }
    }

    pub fn naming(verb: &str, subject: &str) -> Self {
        Self {
            verb: verb.to_string(),
            subject: Some(subject.to_string()),
        }
    }

    /// The whole label as one string, which is what measuring and truncating
    /// work on: the split is a matter of paint, not of length.
    pub fn text(&self) -> String {
        match &self.subject {
            Some(subject) => format!("{} {subject}", self.verb),
            None => self.verb.clone(),
        }
    }

    pub fn width(&self) -> usize {
        self.text().chars().count()
    }
}

/// One row an action would act on, reduced to the facts an availability rule
/// uses. The action layer never holds a view or a tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionTarget {
    pub path: PathBuf,
    pub is_dir: bool,
    /// Whether a directory is expanded. Meaningless on a file, and read only
    /// to decide whether the cursor's row says `Expand` or `Collapse`.
    pub expanded: bool,
    /// Tracked in git but gone from disk: visible, but nothing can act on it.
    pub missing: bool,
}

/// What the action layer is being asked about: the selection in tree order,
/// and the mode it sits in.
#[derive(Clone, Copy, Debug)]
pub struct ActionContext<'a> {
    pub targets: &'a [ActionTarget],
    /// Whether the resolved open command is already the platform opener, in
    /// which case `Open system` would be a duplicate of `Open`.
    pub open_is_platform: bool,
    /// The program the open command runs, for the `Open` row to name.
    pub open_program: &'a str,
    /// The program the platform opener runs — `open`, or `xdg-open`. Named
    /// rather than described: "system" is not a thing anybody runs.
    pub system_program: &'a str,
    /// What the next undo would reverse, already described — `None` when there
    /// is nothing to undo. The action layer holds no history; it is told.
    pub undo: Option<&'a str>,
    /// The row `Enter` would act on. `Open` acts on the **cursor**, not on the
    /// selection, so its label has to describe that row and no other: with a
    /// file and a directory both selected, what `Enter` does depends entirely
    /// on which one the cursor sits in.
    pub cursor: Option<&'a ActionTarget>,
    /// Picker mode, where mutations are disabled by default and `Open` means
    /// *confirm*. No action distinguishes the modes yet — the first that will
    /// is a mutation from `029-add-file-operations` — but the availability rule
    /// is stated over `(targets, mode)` so adding one needs no new plumbing.
    pub picker: bool,
}

impl Action {
    /// Whether the action writes to the filesystem. Mutations are
    /// human-initiated: they never appear in the picker and never on the
    /// socket.
    pub fn mutates(self) -> bool {
        matches!(
            self,
            Action::NewFile | Action::NewFolder | Action::Rename | Action::Delete | Action::Undo
        )
    }

    /// The form of path a copy action writes, or `None` for anything that is
    /// not a copy.
    pub fn copy_form(self) -> Option<CopyForm> {
        match self {
            Action::CopyName => Some(CopyForm::Name),
            Action::CopyRelativePath => Some(CopyForm::Relative),
            Action::CopyAbsolutePath => Some(CopyForm::Absolute),
            _ => None,
        }
    }

    /// The label this action shows **for this selection** — the row describes
    /// what will happen, not a category it belongs to. `Open` is the whole
    /// reason this is not a constant: it runs a program on a file, toggles a
    /// directory, and confirms in the picker.
    pub fn label_in(self, context: ActionContext) -> Label {
        match self {
            Action::Open => {
                // The picker confirms; nothing opens and nothing toggles.
                if context.picker {
                    return Label::plain("Open");
                }
                match context.cursor {
                    // A directory does not open — `Enter` toggles it — so the
                    // row says which way it will go rather than naming an
                    // editor that will not run.
                    Some(target) if target.is_dir => Label::plain(if target.expanded {
                        "Collapse"
                    } else {
                        "Expand"
                    }),
                    _ if context.open_program.is_empty() => Label::plain("Open"),
                    _ => Label::naming("Open with", context.open_program),
                }
            }
            Action::OpenSystem if !context.system_program.is_empty() => {
                Label::naming("Open with", context.system_program)
            }
            // The row names what it would reverse, so it can be trusted
            // without remembering what was just done.
            Action::Undo => match context.undo {
                Some(step) => Label::naming("Undo", step),
                None => Label::plain("Undo"),
            },
            _ => Label::plain(self.label()),
        }
    }

    /// The generic label, without a selection to name anything from.
    pub fn label(self) -> &'static str {
        match self {
            Action::Open => "Open",
            Action::OpenSystem => "Open with the system handler",
            Action::NewFile => "New File\u{2026}",
            Action::NewFolder => "New Folder\u{2026}",
            Action::Rename => "Rename\u{2026}",
            Action::Delete => "Delete",
            Action::CopyName => "Copy Name",
            Action::CopyRelativePath => "Copy Relative Path",
            Action::CopyAbsolutePath => "Copy Absolute Path",
            Action::RevealRootHere => "Reveal Root Here",
            Action::Undo => "Undo",
            // Deliberately not "Reveal in Finder": the manager it lands in is
            // whatever the desktop runs, and on Linux it opens the containing
            // directory rather than selecting the file. A name that promised
            // Finder would be wrong on most of the platforms birch runs on.
            Action::Reveal => "Reveal",
        }
    }

    /// The key that also performs it, shown beside the label so the menu
    /// teaches the keyboard. `None` where no key is bound.
    pub fn accelerator(self) -> Option<&'static str> {
        match self {
            Action::Open => Some("\u{21b5}"),
            Action::OpenSystem => None,
            Action::NewFile => Some("^N"),
            // `Ctrl-Shift-N` is not bindable for the same reason `Ctrl-Shift-C`
            // is not: `Shift` has no representation alongside a control
            // character in the legacy encoding (task 091). `Ctrl-N` with a
            // trailing slash is the keyboard route.
            Action::NewFolder => None,
            Action::Rename => Some("F2"),
            Action::Delete => Some("\u{2326}"),
            // `Ctrl-Shift-C` is not bindable: in the legacy key encoding birch
            // decodes with, `Shift` has no representation alongside a control
            // character, so it is the same byte as `Ctrl-C` — which quits.
            // Task 091 is what would make it possible.
            Action::CopyName | Action::CopyRelativePath | Action::CopyAbsolutePath => None,
            Action::RevealRootHere => None,
            // Free to bind: raw mode clears `ISIG`, so `Ctrl-Z` has never
            // suspended birch and nothing is given up by taking it.
            Action::Undo => Some("^Z"),
            Action::Reveal => None,
        }
    }

    /// Whether the action can be performed on this selection right now. One
    /// rule, consulted by every surface, so a hotkey and a menu entry can never
    /// disagree about what is permitted.
    pub fn available(self, context: ActionContext) -> bool {
        let targets = context.targets;
        if targets.is_empty() || targets.iter().any(|target| target.missing) {
            return false;
        }
        // Mutations are human-initiated and disabled in the picker
        // (`docs/design.md`), which is what `picker` exists to say.
        if context.picker && self.mutates() {
            return false;
        }
        match self {
            // Open acts on the **cursor**, exactly as `Enter` does, so it is
            // offered for any selection: the menu entry and the key agree, and
            // neither opens N files. An action that fans out over the selection
            // arrives with the task that needs it.
            Action::Open => true,
            // Absent when the resolved open command already *is* the platform
            // opener: the menu never shows two rows that do one thing. Absent
            // in the picker too, where `Open` means confirm and a row that
            // launches an application while the picker waits is a mode
            // confusion (task 034).
            Action::OpenSystem => {
                !context.picker && !context.open_is_platform && targets.len() == 1
            }
            // Renaming edits one row, and creating needs one directory to
            // create in — several selected rows name neither.
            Action::NewFile | Action::NewFolder | Action::Rename => targets.len() == 1,
            // Delete is the plural one, and the one trash makes safe
            // (ADR 0031). Arity is per operation, which is what task 074 left
            // to each of them to settle.
            Action::Delete => true,
            // A copy reads paths and writes nothing, so any selection will do.
            Action::CopyName | Action::CopyRelativePath | Action::CopyAbsolutePath => true,
            // Re-rooting is a single-target move by nature, and only a
            // directory can become a root.
            Action::RevealRootHere => targets.len() == 1 && targets[0].is_dir,
            // The only action that ignores the selection entirely: it reverses
            // what birch last did, wherever that was.
            Action::Undo => context.undo.is_some(),
            // Hands off to another application, exactly as `OpenSystem` does,
            // so it is out of the picker for the same reason: a row that
            // launches a file manager while a script waits on a path is a mode
            // confusion (task 034).
            Action::Reveal => !context.picker && targets.len() == 1,
        }
    }
}

/// The menu's shape: ordered groups, drawn with a separator between any two
/// that survive filtering. The groups follow `docs/design.md`; the ones whose
/// actions do not exist yet are empty, and an empty group renders nothing —
/// no row, and no separator (ADR 0029: an unavailable action is absent).
pub const MENU_GROUPS: &[&[Action]] = &[
    &[Action::Open, Action::OpenSystem, Action::Reveal],
    &[Action::Undo],
    &[
        Action::NewFile,
        Action::NewFolder,
        Action::Rename,
        Action::Delete,
    ],
    &[
        Action::CopyName,
        Action::CopyRelativePath,
        Action::CopyAbsolutePath,
    ],
    &[Action::RevealRootHere],
];

/// One drawn row of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuRow {
    Entry(Action),
    /// A rule between two groups. Never highlighted, never invoked.
    Separator,
}

/// Projects the action layer onto a menu for this selection: available actions
/// in declared order, with separators only where they divide two surviving
/// groups. Never begins or ends with a separator.
pub fn menu_rows(context: ActionContext) -> Vec<MenuRow> {
    let mut rows: Vec<MenuRow> = Vec::new();
    for group in MENU_GROUPS {
        let mut available = group
            .iter()
            .copied()
            .filter(|action| action.available(context))
            .peekable();
        if available.peek().is_none() {
            continue;
        }
        if !rows.is_empty() {
            rows.push(MenuRow::Separator);
        }
        rows.extend(available.map(MenuRow::Entry));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(path: &str, is_dir: bool) -> ActionTarget {
        ActionTarget {
            path: PathBuf::from(path),
            is_dir,
            expanded: false,
            missing: false,
        }
    }

    /// The cursor defaults to the first target, which is where it sits whenever
    /// one row is selected — the case every label test but the mixed one wants.
    fn context(targets: &[ActionTarget]) -> ActionContext<'_> {
        ActionContext {
            targets,
            picker: false,
            open_is_platform: false,
            open_program: "nvim",
            system_program: "open",
            undo: None,
            cursor: targets.first(),
        }
    }

    fn entries(targets: &[ActionTarget]) -> Vec<Action> {
        menu_rows(context(targets))
            .into_iter()
            .filter_map(|row| match row {
                MenuRow::Entry(action) => Some(action),
                MenuRow::Separator => None,
            })
            .collect()
    }

    #[test]
    fn a_directory_offers_re_rooting_and_a_file_does_not() {
        assert!(entries(&[target("/r/src", true)]).contains(&Action::RevealRootHere));
        let file = [target("/r/main.rs", false)];
        assert!(
            !entries(&file).contains(&Action::RevealRootHere),
            "a file cannot become a root"
        );
        // The group whose only action vanished takes its separator with it.
        assert_ne!(menu_rows(context(&file)).last(), Some(&MenuRow::Separator));
    }

    #[test]
    fn arity_is_settled_per_operation() {
        let one = [target("/r/a", false)];
        let many = [target("/r/a", false), target("/r/b", false)];
        for action in [Action::NewFile, Action::Rename] {
            assert!(action.available(context(&one)), "{action:?} takes one");
            assert!(
                !action.available(context(&many)),
                "{action:?} takes only one"
            );
        }
        // Re-rooting takes one *directory*, which is stricter again.
        assert!(Action::RevealRootHere.available(context(&[target("/r/src", true)])));
        assert!(!Action::RevealRootHere.available(context(&one)));
        // Delete is the plural one, and the one trash makes safe (ADR 0031).
        assert!(Action::Delete.available(context(&many)));
        // A copy reads paths and writes nothing, so any selection offers it.
        for action in [
            Action::CopyName,
            Action::CopyRelativePath,
            Action::CopyAbsolutePath,
        ] {
            assert!(action.available(context(&many)), "{action:?}");
            assert!(action.copy_form().is_some(), "{action:?}");
        }
        assert!(Action::Open.copy_form().is_none());
    }

    #[test]
    fn open_names_the_program_it_will_run() {
        let file = [target("/r/a.txt", false)];
        let label = Action::Open.label_in(context(&file));
        assert_eq!(label, Label::naming("Open with", "nvim"));
        assert_eq!(label.text(), "Open with nvim");
        // The platform opener is named too: "system" is not a thing anybody
        // runs, and `open` is.
        assert_eq!(
            Action::OpenSystem.label_in(context(&file)).text(),
            "Open with open"
        );

        let picker = ActionContext {
            picker: true,
            ..context(&file)
        };
        assert_eq!(Action::Open.label_in(picker), Label::plain("Open"));
        // Everything else keeps its constant label, with nothing to dim.
        assert_eq!(
            Action::Delete.label_in(context(&file)),
            Label::plain("Delete")
        );
    }

    #[test]
    fn a_directory_says_which_way_enter_will_toggle_it() {
        // `Enter` on a directory expands or collapses it; naming an editor that
        // will not run describes the wrong thing entirely.
        let mut dir = [target("/r/src", true)];
        assert_eq!(Action::Open.label_in(context(&dir)), Label::plain("Expand"));
        dir[0].expanded = true;
        assert_eq!(
            Action::Open.label_in(context(&dir)),
            Label::plain("Collapse")
        );
    }

    #[test]
    fn undo_is_absent_with_nothing_to_reverse_and_names_the_step_otherwise() {
        let file = [target("/r/a.txt", false)];
        assert!(!entries(&file).contains(&Action::Undo), "nothing done yet");

        let with_history = ActionContext {
            undo: Some("Rename of b.txt"),
            ..context(&file)
        };
        assert!(Action::Undo.available(with_history));
        assert_eq!(
            Action::Undo.label_in(with_history).text(),
            "Undo Rename of b.txt"
        );
        // A mutation, so it never appears in the picker.
        let picker = ActionContext {
            picker: true,
            ..with_history
        };
        assert!(!Action::Undo.available(picker));
    }

    #[test]
    fn open_describes_the_cursor_row_not_the_selection() {
        // `Open` acts on the cursor, so with a directory and a file both
        // selected the label depends only on which one the cursor is in.
        let mixed = [target("/r/src", true), target("/r/a.txt", false)];
        let on_dir = ActionContext {
            cursor: mixed.first(),
            ..context(&mixed)
        };
        assert_eq!(Action::Open.label_in(on_dir), Label::plain("Expand"));
        let on_file = ActionContext {
            cursor: mixed.get(1),
            ..context(&mixed)
        };
        assert_eq!(
            Action::Open.label_in(on_file).text(),
            "Open with nvim",
            "the directory in the selection changes nothing"
        );
    }

    #[test]
    fn open_system_is_absent_when_it_would_be_a_duplicate() {
        let file = [target("/r/a.txt", false)];
        assert!(Action::OpenSystem.available(context(&file)));
        // The resolved command already is the platform opener.
        let same = ActionContext {
            open_is_platform: true,
            open_program: "open",
            ..context(&file)
        };
        assert!(!Action::OpenSystem.available(same));
        // ...and it never appears in the picker.
        let picker = ActionContext {
            picker: true,
            ..context(&file)
        };
        assert!(!Action::OpenSystem.available(picker));
    }

    #[test]
    fn no_mutation_is_offered_in_the_picker() {
        let targets = [target("/r/src", true)];
        let picker = ActionContext {
            picker: true,
            ..context(&targets)
        };
        for action in [Action::NewFile, Action::Rename, Action::Delete] {
            assert!(action.mutates(), "{action:?}");
            assert!(!action.available(picker), "{action:?} in the picker");
        }
        // Reading is still fine there.
        for action in [Action::Open, Action::CopyName, Action::RevealRootHere] {
            assert!(!action.mutates(), "{action:?}");
            assert!(action.available(picker), "{action:?} in the picker");
        }
    }

    #[test]
    fn re_rooting_is_single_target_and_nothing_acts_on_a_missing_row() {
        let two = [target("/r/src", true), target("/r/docs", true)];
        assert!(!entries(&two).contains(&Action::RevealRootHere));

        let missing = [ActionTarget {
            path: PathBuf::from("/r/gone.rs"),
            is_dir: false,
            expanded: false,
            missing: true,
        }];
        assert!(menu_rows(context(&missing)).is_empty());
        assert!(menu_rows(context(&[])).is_empty());
    }

    #[test]
    fn a_projection_never_starts_or_ends_with_a_separator() {
        for targets in [
            vec![target("/r/src", true)],
            vec![target("/r/main.rs", false)],
            vec![target("/r/a", true), target("/r/b", false)],
        ] {
            let rows = menu_rows(context(&targets));
            assert_ne!(rows.first(), Some(&MenuRow::Separator));
            assert_ne!(rows.last(), Some(&MenuRow::Separator));
        }
    }
}
