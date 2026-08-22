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
    /// Create a file or directory under the target, inline.
    NewFile,
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

/// One row an action would act on, reduced to the facts an availability rule
/// uses. The action layer never holds a view or a tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionTarget {
    pub path: PathBuf,
    pub is_dir: bool,
    /// Tracked in git but gone from disk: visible, but nothing can act on it.
    pub missing: bool,
}

/// What the action layer is being asked about: the selection in tree order,
/// and the mode it sits in.
#[derive(Clone, Copy, Debug)]
pub struct ActionContext<'a> {
    pub targets: &'a [ActionTarget],
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
        matches!(self, Action::NewFile | Action::Rename | Action::Delete)
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

    /// The menu label. Not the accelerator — that is drawn separately and is
    /// the first thing dropped when the pane is too narrow.
    pub fn label(self) -> &'static str {
        match self {
            Action::Open => "Open",
            Action::NewFile => "New File\u{2026}",
            Action::Rename => "Rename\u{2026}",
            Action::Delete => "Delete",
            Action::CopyName => "Copy Name",
            Action::CopyRelativePath => "Copy Relative Path",
            Action::CopyAbsolutePath => "Copy Absolute Path",
            Action::RevealRootHere => "Reveal Root Here",
        }
    }

    /// The key that also performs it, shown beside the label so the menu
    /// teaches the keyboard. `None` where no key is bound.
    pub fn accelerator(self) -> Option<&'static str> {
        match self {
            Action::Open => Some("\u{21b5}"),
            Action::NewFile => Some("^N"),
            Action::Rename => Some("F2"),
            Action::Delete => Some("\u{2326}"),
            // `Ctrl-Shift-C` is not bindable: in the legacy key encoding birch
            // decodes with, `Shift` has no representation alongside a control
            // character, so it is the same byte as `Ctrl-C` — which quits.
            // Task 091 is what would make it possible.
            Action::CopyName | Action::CopyRelativePath | Action::CopyAbsolutePath => None,
            Action::RevealRootHere => None,
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
            // Renaming edits one row, and creating needs one directory to
            // create in — several selected rows name neither.
            Action::NewFile | Action::Rename => targets.len() == 1,
            // Delete is the plural one, and the one trash makes safe
            // (ADR 0031). Arity is per operation, which is what task 074 left
            // to each of them to settle.
            Action::Delete => true,
            // A copy reads paths and writes nothing, so any selection will do.
            Action::CopyName | Action::CopyRelativePath | Action::CopyAbsolutePath => true,
            // Re-rooting is a single-target move by nature, and only a
            // directory can become a root.
            Action::RevealRootHere => targets.len() == 1 && targets[0].is_dir,
        }
    }
}

/// The menu's shape: ordered groups, drawn with a separator between any two
/// that survive filtering. The groups follow `docs/design.md`; the ones whose
/// actions do not exist yet are empty, and an empty group renders nothing —
/// no row, and no separator (ADR 0029: an unavailable action is absent).
pub const MENU_GROUPS: &[&[Action]] = &[
    &[Action::Open],
    &[Action::NewFile, Action::Rename, Action::Delete],
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
            missing: false,
        }
    }

    fn context(targets: &[ActionTarget]) -> ActionContext<'_> {
        ActionContext {
            targets,
            picker: false,
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
    fn no_mutation_is_offered_in_the_picker() {
        let targets = [target("/r/src", true)];
        let picker = ActionContext {
            targets: &targets,
            picker: true,
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
