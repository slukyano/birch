//! Undo (task 093).
//!
//! **Session-only, and a stack.** Every mutation birch performs pushes a step;
//! `Ctrl-Z` reverses the most recent one. Nothing is written to disk: a journal
//! of filesystem mutations that survives restarts is the different product
//! [ADR 0031](../../docs/adr/0031-delete-goes-to-trash.md) already declined to
//! become, and the mistake people actually make is the one they notice at once.
//!
//! **Delete is the asymmetric one, and it is honest about it.** Its way back
//! lives outside birch, in the platform trash, and the `trash` crate's
//! `os_limited::restore_all` is compiled only on Windows and Freedesktop
//! systems — not on macOS. So a delete pushes a step that cannot be reversed,
//! and reversing it says where the files went instead of quietly reversing
//! whatever came *before* the delete. A destructive surprise is the one thing
//! undo must never be.
//!
//! **Every step is validated against the filesystem when it runs, never when it
//! is recorded.** The world moves underneath: the old name may be taken again,
//! the file may be gone, another process may have moved it. A step that cannot
//! be reversed says why and reverses nothing.

use std::path::{Path, PathBuf};

use crate::ops::{self, OpError};
use crate::tree::TreeDelta;

/// How many steps are kept. Deep enough that the stack is never the reason an
/// undo fails, shallow enough to stay a bounded amount of memory.
const DEPTH: usize = 100;

/// One thing birch did that can be spoken about in the past tense.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Step {
    /// Paths that moved. One pair for a rename, several for a drag that
    /// carried a multi-row selection.
    Moved { pairs: Vec<(PathBuf, PathBuf)> },
    /// A path that was created. Reversing it **trashes** rather than unlinks:
    /// the file may have been written to since birch made it, and destroying
    /// work birch never saw is not what undo means.
    Created { path: PathBuf },
    /// Paths that went to the trash, which is where they stay.
    Trashed { what: String },
}

impl Step {
    /// What the menu row and the status line call this step. Phrased as the
    /// thing that happened, so `Undo` reads as a sentence against it.
    pub fn describe(&self) -> String {
        match self {
            Step::Moved { pairs } if pairs.len() == 1 => {
                let (from, to) = &pairs[0];
                // A rename keeps its parent; anything else is a move.
                if from.parent() == to.parent() {
                    format!("Rename of {}", name(to))
                } else {
                    format!("Move of {}", name(to))
                }
            }
            Step::Moved { pairs } => format!("Move of {} items", pairs.len()),
            Step::Created { path } => format!("New {}", name(path)),
            Step::Trashed { what } => format!("Delete of {what}"),
        }
    }
}

/// What reversing a step did.
pub struct Undone {
    pub deltas: Vec<TreeDelta>,
    /// What to say about it, whether it worked or not.
    pub message: String,
    /// Set when the step could not be fully reversed. Any `deltas` above still
    /// happened — a partial undo is reported, never silent.
    pub failed: Option<OpError>,
}

/// The stack. Session-only: dropped with the process, exactly as designed.
#[derive(Debug, Default)]
pub struct History {
    steps: Vec<Step>,
}

impl History {
    pub fn push(&mut self, step: Step) {
        self.steps.push(step);
        if self.steps.len() > DEPTH {
            self.steps.remove(0);
        }
    }

    /// The step `Ctrl-Z` would reverse, for the menu row to name.
    pub fn next(&self) -> Option<&Step> {
        self.steps.last()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Reverses the most recent step, consuming it either way — including when
    /// it fails. A step that stayed on the stack after refusing would refuse
    /// again on the next `Ctrl-Z`, and undo would be permanently stuck on it.
    pub fn undo(&mut self) -> Option<Undone> {
        let step = self.steps.pop()?;
        Some(match step {
            Step::Trashed { what } => Undone {
                deltas: Vec::new(),
                message: format!("{what} is in the trash — restore it from there"),
                failed: None,
            },
            Step::Created { path } => {
                let removal = ops::trash(std::slice::from_ref(&path));
                Undone {
                    message: match &removal.failed {
                        Some(e) => format!("Could not undo: {e}"),
                        None => format!("Undid New {} — it is in the trash", name(&path)),
                    },
                    deltas: removal.deltas,
                    failed: removal.failed,
                }
            }
            Step::Moved { pairs } => {
                let mut deltas = Vec::new();
                let mut failed = None;
                // Reversed, so a chain of moves unwinds in the order it was
                // made. Stops at the first refusal rather than pressing on:
                // the rest of the batch may depend on the one that failed.
                for (from, to) in pairs.iter().rev() {
                    match ops::move_to(to, from) {
                        Ok(more) => deltas.extend(more),
                        Err(e) => {
                            failed = Some(e);
                            break;
                        }
                    }
                }
                Undone {
                    message: match &failed {
                        Some(e) => format!("Could not undo: {e}"),
                        None => format!("Undid {} of {}", verb(&pairs), described(&pairs)),
                    },
                    deltas,
                    failed,
                }
            }
        })
    }
}

fn verb(pairs: &[(PathBuf, PathBuf)]) -> &'static str {
    match pairs.first() {
        Some((from, to)) if from.parent() == to.parent() => "the rename",
        _ => "the move",
    }
}

fn described(pairs: &[(PathBuf, PathBuf)]) -> String {
    match pairs {
        [(_, to)] => name(to),
        _ => format!("{} items", pairs.len()),
    }
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("birch-undo-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_rename_goes_back_to_the_name_it_had() {
        let root = scratch("rename");
        std::fs::write(root.join("b.txt"), b"x").unwrap();
        let mut history = History::default();
        history.push(Step::Moved {
            pairs: vec![(root.join("a.txt"), root.join("b.txt"))],
        });

        let undone = history.undo().expect("there was a step");
        assert!(undone.failed.is_none(), "{}", undone.message);
        assert!(root.join("a.txt").exists() && !root.join("b.txt").exists());
        assert!(history.is_empty(), "the step was consumed");
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_step_the_world_moved_under_refuses_and_says_why() {
        let root = scratch("taken");
        // The file is back where it was moved to, but the old name is taken
        // again — by something birch knows nothing about.
        std::fs::write(root.join("b.txt"), b"x").unwrap();
        std::fs::write(root.join("a.txt"), b"someone else's").unwrap();
        let mut history = History::default();
        history.push(Step::Moved {
            pairs: vec![(root.join("a.txt"), root.join("b.txt"))],
        });

        let undone = history.undo().expect("there was a step");
        assert!(undone.failed.is_some(), "it refused");
        assert!(undone.message.starts_with("Could not undo"));
        assert_eq!(
            std::fs::read(root.join("a.txt")).unwrap(),
            b"someone else's",
            "and reversed nothing"
        );
        assert!(
            history.is_empty(),
            "a refused step is still consumed, or undo would be stuck on it forever"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_delete_says_where_the_files_went_instead_of_reversing_something_else() {
        // The hazard this exists to prevent: undo after a delete quietly
        // reversing the *rename* before it.
        let mut history = History::default();
        history.push(Step::Moved {
            pairs: vec![("/r/a.txt".into(), "/r/b.txt".into())],
        });
        history.push(Step::Trashed {
            what: "notes.md".into(),
        });

        let undone = history.undo().expect("there was a step");
        assert!(undone.deltas.is_empty() && undone.failed.is_none());
        assert!(
            undone.message.contains("trash"),
            "it says where they went: {}",
            undone.message
        );
        assert!(
            matches!(history.next(), Some(Step::Moved { .. })),
            "and the rename is still there for the next Ctrl-Z"
        );
    }

    #[test]
    fn the_stack_is_bounded() {
        let mut history = History::default();
        for i in 0..DEPTH + 10 {
            history.push(Step::Created {
                path: PathBuf::from(format!("/r/{i}")),
            });
        }
        assert_eq!(history.steps.len(), DEPTH);
        assert_eq!(
            history.next(),
            Some(&Step::Created {
                path: PathBuf::from(format!("/r/{}", DEPTH + 9))
            }),
            "the newest step survives"
        );
    }

    #[test]
    fn a_step_names_what_it_would_reverse() {
        assert_eq!(
            Step::Moved {
                pairs: vec![("/r/a.txt".into(), "/r/b.txt".into())]
            }
            .describe(),
            "Rename of b.txt"
        );
        assert_eq!(
            Step::Moved {
                pairs: vec![("/r/a.txt".into(), "/r/sub/a.txt".into())]
            }
            .describe(),
            "Move of a.txt"
        );
        assert_eq!(
            Step::Created {
                path: "/r/new.txt".into()
            }
            .describe(),
            "New new.txt"
        );
    }
}
