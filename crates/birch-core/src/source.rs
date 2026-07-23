//! The source contract (see ADR 0004): a source runs on a worker thread,
//! receives [`SourceCmd`]s over an mpsc channel, and emits [`SourceEvent`]s
//! into the app's unified event channel. Sources speak real paths only.

use std::path::PathBuf;

use crate::tree::TreeDelta;

/// Commands the app sends to a source.
#[derive(Clone, Debug)]
pub enum SourceCmd {
    /// Load the immediate children of a directory (one level, lazy).
    Expand(PathBuf),
}

/// Events a source emits.
#[derive(Clone, Debug)]
pub enum SourceEvent {
    /// A batch of tree deltas to apply.
    Deltas(Vec<TreeDelta>),
    /// A human-readable status message (e.g. a read error).
    Message(String),
}
