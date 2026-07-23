//! birch-core: the real tree, sources-as-delta-streams, watcher, git status,
//! search, and file ops.
//!
//! This crate must build without ratatui — the crate boundary
//! compiler-enforces the real-tree/render split (see `docs/design.md`).

pub mod files_source;
pub mod git;
pub mod open_cmd;
pub mod persist;
pub mod protocol;
pub mod search;
pub mod settings;
pub mod source;
pub mod tree;
pub mod watcher;

pub use git::{FileStatus, GitState};
pub use open_cmd::{OpenCmd, OpenMode};
pub use settings::Settings;
pub use source::{SourceCmd, SourceEvent};
pub use tree::{Entry, Node, NodeId, NodeKind, Tree, TreeDelta};
