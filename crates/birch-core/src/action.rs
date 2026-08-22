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
    /// Re-root the tree at the selected directory.
    RevealRootHere,
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
    /// The menu label. Not the accelerator — that is drawn separately and is
    /// the first thing dropped when the pane is too narrow.
    pub fn label(self) -> &'static str {
        match self {
            Action::Open => "Open",
            Action::RevealRootHere => "Reveal Root Here",
        }
    }

    /// The key that also performs it, shown beside the label so the menu
    /// teaches the keyboard. `None` where no key is bound.
    pub fn accelerator(self) -> Option<&'static str> {
        match self {
            Action::Open => Some("\u{21b5}"),
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
        match self {
            // Open acts on the **cursor**, exactly as `Enter` does, so it is
            // offered for any selection: the menu entry and the key agree, and
            // neither opens N files. An action that fans out over the selection
            // arrives with the task that needs it.
            Action::Open => true,
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
    // New File… / Rename… / Delete — 029-add-file-operations.
    &[],
    // Copy Name / Relative Path / Absolute Path — 028-add-copy-paths.
    &[],
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

    #[test]
    fn a_directory_offers_both_actions_and_a_file_only_open() {
        let dir = [target("/r/src", true)];
        assert_eq!(
            menu_rows(context(&dir)),
            [
                MenuRow::Entry(Action::Open),
                MenuRow::Separator,
                MenuRow::Entry(Action::RevealRootHere),
            ]
        );
        // A file cannot become a root, so its group vanishes — and the
        // separator that would have divided it goes with it.
        let file = [target("/r/main.rs", false)];
        assert_eq!(menu_rows(context(&file)), [MenuRow::Entry(Action::Open)]);
    }

    #[test]
    fn re_rooting_is_single_target_and_nothing_acts_on_a_missing_row() {
        let two = [target("/r/src", true), target("/r/docs", true)];
        assert_eq!(menu_rows(context(&two)), [MenuRow::Entry(Action::Open)]);

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
