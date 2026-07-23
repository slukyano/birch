//! The real tree: nodes keyed by real path, mutated only by applying
//! [`TreeDelta`]s emitted by sources.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Handle to a node in the arena. Valid only until the next `Removed` delta:
/// removal frees the slot and a later insertion may reuse it, so ids must be
/// re-resolved from paths across `apply` calls, never stored long-term.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NodeId(usize);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeKind {
    Dir,
    File,
    SymlinkDir,
    SymlinkFile,
}

impl NodeKind {
    pub fn is_dir(self) -> bool {
        matches!(self, NodeKind::Dir | NodeKind::SymlinkDir)
    }
}

/// A directory entry as observed by a source: name + kind snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub kind: NodeKind,
}

/// The delta language sources speak. Real paths throughout.
#[derive(Clone, Debug)]
pub enum TreeDelta {
    /// The authoritative one-level listing of `dir` (ADR 0006). The tree
    /// reconciles: inserts new names, updates kinds, removes subtrees of
    /// vanished names — preserving surviving children's state. Marks `dir`
    /// loaded.
    Snapshot { dir: PathBuf, entries: Vec<Entry> },
    /// Entries observed under `parent`. Merge-upsert semantics: new names are
    /// inserted, existing names may change kind. Marks `parent` as loaded.
    Added {
        parent: PathBuf,
        entries: Vec<Entry>,
    },
    /// The path no longer exists; removes the whole subtree.
    Removed { path: PathBuf },
    /// The node at `path` changed kind.
    Updated { path: PathBuf, kind: NodeKind },
}

#[derive(Debug)]
pub struct Node {
    pub name: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    /// `None` = children never loaded (lazy). `Some` may be empty.
    children: Option<Vec<NodeId>>,
    pub expanded: bool,
}

impl Node {
    pub fn is_loaded(&self) -> bool {
        self.children.is_some()
    }

    /// Loaded children in sorted order; empty slice when unloaded.
    pub fn children(&self) -> &[NodeId] {
        self.children.as_deref().unwrap_or(&[])
    }
}

pub struct Tree {
    nodes: Vec<Option<Node>>,
    free: Vec<usize>,
    index: HashMap<PathBuf, NodeId>,
    root: NodeId,
    files_first: bool,
}

impl Tree {
    pub fn new(root_path: PathBuf, files_first: bool) -> Self {
        let root_name = root_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| root_path.display().to_string());
        let root_node = Node {
            name: root_name,
            path: root_path.clone(),
            kind: NodeKind::Dir,
            parent: None,
            children: None,
            expanded: false,
        };
        let mut index = HashMap::new();
        index.insert(root_path, NodeId(0));
        Self {
            nodes: vec![Some(root_node)],
            free: Vec::new(),
            index,
            root: NodeId(0),
            files_first,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn get(&self, id: NodeId) -> &Node {
        self.nodes[id.0].as_ref().expect("live node id")
    }

    pub fn id_of(&self, path: &Path) -> Option<NodeId> {
        self.index.get(path).copied()
    }

    pub fn node_at(&self, path: &Path) -> Option<&Node> {
        self.id_of(path).map(|id| self.get(id))
    }

    /// Changes the sort grouping at runtime and re-sorts every loaded dir.
    pub fn set_files_first(&mut self, files_first: bool) {
        if self.files_first == files_first {
            return;
        }
        self.files_first = files_first;
        let dirs: Vec<NodeId> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.as_ref().is_some_and(|n| n.is_loaded()))
            .map(|(i, _)| NodeId(i))
            .collect();
        for dir in dirs {
            self.sort_children(dir);
        }
    }

    /// Every expanded dir in the tree (persistence uses this snapshot).
    pub fn expanded_dirs(&self) -> Vec<PathBuf> {
        self.nodes
            .iter()
            .flatten()
            .filter(|n| n.expanded && n.kind.is_dir())
            .map(|n| n.path.clone())
            .collect()
    }

    /// Sets the expansion flag; returns false if the path is unknown.
    pub fn set_expanded(&mut self, path: &Path, expanded: bool) -> bool {
        match self.id_of(path) {
            Some(id) => {
                self.nodes[id.0].as_mut().expect("live node id").expanded = expanded;
                true
            }
            None => false,
        }
    }

    pub fn apply(&mut self, delta: TreeDelta) {
        match delta {
            TreeDelta::Snapshot { dir, entries } => self.apply_snapshot(&dir, entries),
            TreeDelta::Added { parent, entries } => self.apply_added(&parent, entries),
            TreeDelta::Removed { path } => self.apply_removed(&path),
            TreeDelta::Updated { path, kind } => {
                if let Some(id) = self.id_of(&path) {
                    self.change_kind(id, kind);
                    if let Some(parent) = self.get(id).parent {
                        self.sort_children(parent);
                    }
                }
            }
        }
    }

    fn apply_snapshot(&mut self, dir: &Path, entries: Vec<Entry>) {
        let Some(dir_id) = self.id_of(dir) else {
            return; // dir vanished before the delta arrived; drop it
        };
        // Remove children whose names are gone from the listing.
        let keep: std::collections::HashSet<&str> =
            entries.iter().map(|e| e.name.as_str()).collect();
        let stale: Vec<NodeId> = self
            .get(dir_id)
            .children()
            .iter()
            .copied()
            .filter(|&c| !keep.contains(self.get(c).name.as_str()))
            .collect();
        for id in stale {
            if let Some(children) = self.nodes[dir_id.0]
                .as_mut()
                .expect("live node id")
                .children
                .as_mut()
            {
                children.retain(|&c| c != id);
            }
            self.remove_subtree(id);
        }
        // Merge the rest (insert new, update kinds) and mark loaded.
        self.apply_added(dir, entries);
    }

    fn apply_added(&mut self, parent: &Path, entries: Vec<Entry>) {
        let Some(parent_id) = self.id_of(parent) else {
            return; // parent vanished before the delta arrived; drop it
        };
        for entry in entries {
            let path = parent.join(&entry.name);
            match self.index.get(&path) {
                Some(&id) => self.change_kind(id, entry.kind),
                None => {
                    let id = self.alloc(Node {
                        name: entry.name,
                        path: path.clone(),
                        kind: entry.kind,
                        parent: Some(parent_id),
                        children: None,
                        expanded: false,
                    });
                    self.index.insert(path, id);
                    let parent_node = self.nodes[parent_id.0].as_mut().expect("live node id");
                    parent_node.children.get_or_insert_with(Vec::new).push(id);
                }
            }
        }
        // An Added delta is a load observation even when empty.
        let parent_node = self.nodes[parent_id.0].as_mut().expect("live node id");
        parent_node.children.get_or_insert_with(Vec::new);
        self.sort_children(parent_id);
    }

    fn apply_removed(&mut self, path: &Path) {
        let Some(id) = self.id_of(path) else { return };
        if id == self.root {
            return; // the root cannot be removed
        }
        if let Some(parent_id) = self.get(id).parent
            && let Some(children) = self.nodes[parent_id.0]
                .as_mut()
                .expect("live node id")
                .children
                .as_mut()
        {
            children.retain(|&c| c != id);
        }
        self.remove_subtree(id);
    }

    /// Changes a node's kind. Crossing the dir/file boundary drops the old
    /// children and expansion — a dir recreated at a former file's path (or
    /// vice versa) must not inherit a stale subtree or loaded state.
    fn change_kind(&mut self, id: NodeId, kind: NodeKind) {
        let node = self.nodes[id.0].as_mut().expect("live node id");
        let crossed = node.kind.is_dir() != kind.is_dir();
        node.kind = kind;
        if !crossed {
            return;
        }
        node.expanded = false;
        let children = node.children.take().unwrap_or_default();
        for child in children {
            self.remove_subtree(child);
        }
    }

    fn remove_subtree(&mut self, id: NodeId) {
        let node = self.nodes[id.0].take().expect("live node id");
        self.index.remove(&node.path);
        self.free.push(id.0);
        for child in node.children.unwrap_or_default() {
            self.remove_subtree(child);
        }
    }

    fn alloc(&mut self, node: Node) -> NodeId {
        match self.free.pop() {
            Some(slot) => {
                self.nodes[slot] = Some(node);
                NodeId(slot)
            }
            None => {
                self.nodes.push(Some(node));
                NodeId(self.nodes.len() - 1)
            }
        }
    }

    fn sort_children(&mut self, parent: NodeId) {
        let Some(mut children) = self.nodes[parent.0]
            .as_mut()
            .expect("live node id")
            .children
            .take()
        else {
            return;
        };
        let files_first = self.files_first;
        children.sort_by(|&a, &b| {
            let (a, b) = (self.get(a), self.get(b));
            let (da, db) = (a.kind.is_dir(), b.kind.is_dir());
            let group = if files_first {
                da.cmp(&db)
            } else {
                db.cmp(&da)
            };
            group
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.name.cmp(&b.name))
        });
        self.nodes[parent.0]
            .as_mut()
            .expect("live node id")
            .children = Some(children);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: NodeKind) -> Entry {
        Entry {
            name: name.into(),
            kind,
        }
    }

    fn names(tree: &Tree, id: NodeId) -> Vec<String> {
        tree.get(id)
            .children()
            .iter()
            .map(|&c| tree.get(c).name.clone())
            .collect()
    }

    #[test]
    fn added_sorts_dirs_first_then_case_insensitive() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![
                entry("beta.txt", NodeKind::File),
                entry("Alpha", NodeKind::Dir),
                entry("alpha.txt", NodeKind::File),
                entry("zeta", NodeKind::SymlinkDir),
            ],
        });
        assert_eq!(
            names(&tree, tree.root()),
            ["Alpha", "zeta", "alpha.txt", "beta.txt"]
        );
    }

    #[test]
    fn files_first_flips_grouping() {
        let mut tree = Tree::new(PathBuf::from("/r"), true);
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![entry("dir", NodeKind::Dir), entry("file", NodeKind::File)],
        });
        assert_eq!(names(&tree, tree.root()), ["file", "dir"]);
    }

    #[test]
    fn added_merges_and_marks_loaded() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        assert!(!tree.get(tree.root()).is_loaded());
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![entry("a", NodeKind::File)],
        });
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![entry("a", NodeKind::File), entry("b", NodeKind::File)],
        });
        assert!(tree.get(tree.root()).is_loaded());
        assert_eq!(names(&tree, tree.root()), ["a", "b"]);
    }

    #[test]
    fn empty_added_marks_loaded() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![],
        });
        assert!(tree.get(tree.root()).is_loaded());
        assert!(tree.get(tree.root()).children().is_empty());
    }

    #[test]
    fn removed_drops_subtree_and_index() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![entry("dir", NodeKind::Dir), entry("keep", NodeKind::File)],
        });
        tree.apply(TreeDelta::Added {
            parent: "/r/dir".into(),
            entries: vec![entry("inner", NodeKind::File)],
        });
        tree.apply(TreeDelta::Removed {
            path: "/r/dir".into(),
        });
        assert_eq!(names(&tree, tree.root()), ["keep"]);
        assert!(tree.id_of(Path::new("/r/dir")).is_none());
        assert!(tree.id_of(Path::new("/r/dir/inner")).is_none());
    }

    #[test]
    fn added_for_unknown_parent_is_dropped() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Added {
            parent: "/r/ghost".into(),
            entries: vec![entry("x", NodeKind::File)],
        });
        assert!(tree.id_of(Path::new("/r/ghost/x")).is_none());
    }

    #[test]
    fn updated_changes_kind_and_resorts() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Added {
            parent: "/r".into(),
            entries: vec![entry("a", NodeKind::File), entry("b", NodeKind::Dir)],
        });
        tree.apply(TreeDelta::Updated {
            path: "/r/a".into(),
            kind: NodeKind::Dir,
        });
        assert_eq!(names(&tree, tree.root()), ["a", "b"]);
    }

    #[test]
    fn kind_change_across_dir_file_boundary_drops_subtree() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![entry("x", NodeKind::Dir)],
        });
        tree.apply(TreeDelta::Snapshot {
            dir: "/r/x".into(),
            entries: vec![entry("inner", NodeKind::File)],
        });
        tree.set_expanded(Path::new("/r/x"), true);

        // x becomes a file: subtree, loaded state, and expansion must go.
        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![entry("x", NodeKind::File)],
        });
        assert!(tree.id_of(Path::new("/r/x/inner")).is_none());
        let x = tree.node_at(Path::new("/r/x")).unwrap();
        assert!(!x.expanded && !x.is_loaded());

        // Becoming a dir again starts fresh — unloaded, no ghost children.
        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![entry("x", NodeKind::Dir)],
        });
        let x = tree.node_at(Path::new("/r/x")).unwrap();
        assert!(!x.is_loaded());
        assert!(x.children().is_empty());
    }

    #[test]
    fn snapshot_reconciles_and_preserves_surviving_state() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![
                entry("keep", NodeKind::Dir),
                entry("gone", NodeKind::File),
                entry("becomes-dir", NodeKind::File),
            ],
        });
        // Load and expand the surviving dir.
        tree.apply(TreeDelta::Snapshot {
            dir: "/r/keep".into(),
            entries: vec![entry("inner", NodeKind::File)],
        });
        tree.set_expanded(Path::new("/r/keep"), true);

        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![
                entry("keep", NodeKind::Dir),
                entry("becomes-dir", NodeKind::Dir),
                entry("new", NodeKind::File),
            ],
        });
        assert_eq!(names(&tree, tree.root()), ["becomes-dir", "keep", "new"]);
        assert!(tree.id_of(Path::new("/r/gone")).is_none());
        let keep = tree.node_at(Path::new("/r/keep")).unwrap();
        assert!(keep.expanded && keep.is_loaded());
        assert!(tree.id_of(Path::new("/r/keep/inner")).is_some());
        assert_eq!(
            tree.node_at(Path::new("/r/becomes-dir")).unwrap().kind,
            NodeKind::Dir
        );
    }

    #[test]
    fn empty_snapshot_clears_children() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![entry("a", NodeKind::File)],
        });
        tree.apply(TreeDelta::Snapshot {
            dir: "/r".into(),
            entries: vec![],
        });
        assert!(tree.get(tree.root()).children().is_empty());
        assert!(tree.get(tree.root()).is_loaded());
        assert!(tree.id_of(Path::new("/r/a")).is_none());
    }

    #[test]
    fn expansion_flag_round_trips() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        assert!(tree.set_expanded(Path::new("/r"), true));
        assert!(tree.get(tree.root()).expanded);
        assert!(!tree.set_expanded(Path::new("/r/none"), true));
    }
}
