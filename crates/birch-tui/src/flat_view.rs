//! The view-model (ADR 0003): flattens the expanded real tree into visible
//! rows, applies visibility settings, git decorations (ADR 0005), and chain
//! compaction (ADR 0007), and owns selection + scroll. Selection is keyed by
//! real path, so rows appearing or disappearing above it cannot move it.
//! Pure logic — no ratatui types.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use birch_core::git::GitState;
use birch_core::search::Match;
use birch_core::{FileStatus, NodeId, NodeKind, Settings, Tree, settings};

#[derive(Clone, Debug)]
pub struct Row {
    pub path: PathBuf,
    /// Display label; `a/b/c` for a compacted chain.
    pub name: String,
    pub kind: NodeKind,
    /// 0 for the root row; its immediate children are 1.
    pub depth: usize,
    pub expanded: bool,
    pub loaded: bool,
    /// Real paths of the chain members (head..=tail); empty for plain rows.
    pub chain: Vec<PathBuf>,
    pub status: Option<FileStatus>,
    pub ignored: bool,
    /// Deleted-but-tracked: rendered from git state, absent on disk.
    pub missing: bool,
    /// Active search: `Some(true)` = match (highlight), `Some(false)` = dim.
    pub search: Option<bool>,
    /// Char positions inside `name` to light up (empty for whole-row
    /// highlight — e.g. path-mode matches whose characters are off-screen).
    pub match_indices: Vec<u32>,
    /// Dim text after the label (the root row carries its full path).
    pub annotation: Option<String>,
}

/// What a navigation action asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavEffect {
    None,
    /// Ask the source to load a directory's children.
    RequestExpand(PathBuf),
    /// Open a file.
    Open(PathBuf),
    /// Show a transient status message.
    Message(String),
}

/// Decorations applied at flatten time: git state and the active search's
/// matches (path → matched char indices into the simple name; an empty list
/// means "hit, no char detail" — path-mode matches).
#[derive(Default, Clone, Copy)]
pub struct Decor<'a> {
    pub git: Option<&'a GitState>,
    pub matched: Option<&'a HashMap<PathBuf, Vec<u32>>>,
    /// Home dir for the root annotation's `~` abbreviation (computed once by
    /// the app; `None` renders the full path).
    pub home: Option<&'a std::path::Path>,
    /// Chains split on demand (ADR 0014): a dir in this set neither starts
    /// nor extends a compact chain. Owned by `FlatView`.
    pub split: Option<&'a HashSet<PathBuf>>,
}

struct Ctx<'a> {
    tree: &'a Tree,
    s: &'a Settings,
    git: Option<&'a GitState>,
    matched: Option<&'a HashMap<PathBuf, Vec<u32>>>,
    split: Option<&'a HashSet<PathBuf>>,
}

impl Ctx<'_> {
    fn search_flag(&self, hit: bool) -> Option<bool> {
        self.matched.map(|_| hit)
    }

    fn is_hit(&self, path: &std::path::Path) -> bool {
        self.matched.is_some_and(|m| m.contains_key(path))
    }

    fn hit_indices(&self, path: &std::path::Path) -> Vec<u32> {
        self.matched
            .and_then(|m| m.get(path))
            .cloned()
            .unwrap_or_default()
    }

    fn is_ignored(&self, path: &std::path::Path) -> bool {
        self.git.is_some_and(|g| g.is_ignored(path))
    }

    fn is_split(&self, path: &std::path::Path) -> bool {
        self.split.is_some_and(|s| s.contains(path))
    }

    fn name_visible(&self, name: &str) -> bool {
        if settings::is_noise(name) && !self.s.show_noise {
            return false;
        }
        if settings::is_hidden(name) && !self.s.show_hidden {
            return false;
        }
        true
    }
}

enum Child {
    Real(NodeId),
    /// A deleted-but-tracked file name (no node exists).
    Missing(String),
}

/// The visible children of a loaded dir — real entries plus synthetic
/// deleted-but-tracked files, merged in display sort order.
fn visible_children(ctx: &Ctx, dir_id: NodeId) -> Vec<Child> {
    let dir = ctx.tree.get(dir_id);
    let mut items: Vec<(bool, String, Child)> = Vec::new();
    for &child in dir.children() {
        let node = ctx.tree.get(child);
        if !ctx.name_visible(&node.name) {
            continue;
        }
        if !ctx.s.show_ignored && ctx.is_ignored(&node.path) {
            continue;
        }
        items.push((
            node.kind.is_dir(),
            node.name.to_lowercase(),
            Child::Real(child),
        ));
    }
    if let Some(git) = ctx.git {
        for name in git.deleted_in(&dir.path) {
            if !ctx.name_visible(name) {
                continue;
            }
            // A staged delete followed by re-creation produces both a real
            // entry and a deleted record; the real one wins.
            if ctx.tree.id_of(&dir.path.join(name)).is_some() {
                continue;
            }
            items.push((false, name.to_lowercase(), Child::Missing(name.clone())));
        }
    }
    let dirs_first = !ctx.s.files_first;
    items.sort_by(|(a_dir, a_name, _), (b_dir, b_name, _)| {
        let group = if dirs_first {
            b_dir.cmp(a_dir)
        } else {
            a_dir.cmp(b_dir)
        };
        group.then_with(|| a_name.cmp(b_name))
    });
    items.into_iter().map(|(_, _, child)| child).collect()
}

/// Flattens the tree into visible rows. The root is the first row (its
/// children nest below it); traversal descends only into expanded
/// directories. The root never joins a compact chain and is exempt from
/// visibility filters (a dot-dir root still shows).
pub fn visible_rows(tree: &Tree, s: &Settings, decor: Decor) -> Vec<Row> {
    let ctx = Ctx {
        tree,
        s,
        git: decor.git,
        matched: decor.matched,
        split: decor.split,
    };
    let mut rows = Vec::new();
    let root = tree.get(tree.root());
    rows.push(Row {
        path: root.path.clone(),
        name: root.name.clone(),
        kind: NodeKind::Dir,
        depth: 0,
        expanded: root.expanded,
        loaded: root.is_loaded(),
        chain: Vec::new(),
        status: ctx.git.and_then(|git| git.dir_status(&root.path)),
        ignored: false,
        missing: false,
        search: ctx.search_flag(ctx.is_hit(&root.path)),
        match_indices: ctx.hit_indices(&root.path),
        annotation: Some(abbreviate_home(&root.path, std::env::home_dir().as_deref())),
    });
    if root.expanded {
        push_children(&ctx, tree.root(), 1, &mut rows);
    }
    rows
}

fn push_children(ctx: &Ctx, dir_id: NodeId, depth: usize, rows: &mut Vec<Row>) {
    let dir_path = ctx.tree.get(dir_id).path.clone();
    for child in visible_children(ctx, dir_id) {
        match child {
            Child::Missing(name) => {
                let path = dir_path.join(&name);
                let search = ctx.search_flag(ctx.is_hit(&path));
                let match_indices = ctx.hit_indices(&path);
                rows.push(Row {
                    path,
                    name,
                    kind: NodeKind::File,
                    depth,
                    expanded: false,
                    loaded: false,
                    chain: Vec::new(),
                    status: Some(FileStatus::Deleted),
                    ignored: false,
                    missing: true,
                    search,
                    match_indices,
                    annotation: None,
                });
            }
            Child::Real(id) => push_node(ctx, id, depth, rows),
        }
    }
}

fn push_node(ctx: &Ctx, id: NodeId, depth: usize, rows: &mut Vec<Row>) {
    let node = ctx.tree.get(id);
    // Chain compaction (ADR 0007): extend while each member's only visible
    // child is a single dir. An unloaded tail ends the chain and may extend
    // in place once its peek-load lands. Symlinked dirs never join a chain
    // (a `dir -> .` symlink would otherwise grow the chain forever), a chain
    // never crosses into gitignored territory, and a dir split on demand
    // (ADR 0014) neither starts nor extends one.
    let mut members = vec![id];
    if ctx.s.compact
        && node.kind == NodeKind::Dir
        && !ctx.is_ignored(&node.path)
        && !ctx.is_split(&node.path)
    {
        loop {
            let cur = *members.last().expect("chain is never empty");
            if !ctx.tree.get(cur).is_loaded() {
                break;
            }
            let children = visible_children(ctx, cur);
            let [Child::Real(only)] = children.as_slice() else {
                break;
            };
            let only_node = ctx.tree.get(*only);
            if only_node.kind != NodeKind::Dir
                || ctx.is_ignored(&only_node.path)
                || ctx.is_split(&only_node.path)
            {
                break;
            }
            members.push(*only);
        }
    }
    let tail = *members.last().expect("chain is never empty");
    let tail_node = ctx.tree.get(tail);
    let label = members
        .iter()
        .map(|&m| ctx.tree.get(m).name.as_str())
        .collect::<Vec<_>>()
        .join("/");
    let status = match ctx.git {
        Some(git) if tail_node.kind.is_dir() => git.dir_status(&node.path),
        Some(git) => git.status_of(&node.path),
        None => None,
    };
    let hit =
        ctx.is_hit(&tail_node.path) || members.iter().any(|&m| ctx.is_hit(&ctx.tree.get(m).path));
    // Char highlighting: name-mode indices are relative to each member's
    // simple name; every hit member's segment lights up at its offset within
    // the joined label (ADR 0013 — the matched characters are on screen).
    let mut match_indices: Vec<u32> = Vec::new();
    let mut segment_offset = 0u32;
    for &member in &members {
        let member_node = ctx.tree.get(member);
        if ctx.is_hit(&member_node.path) {
            match_indices.extend(
                ctx.hit_indices(&member_node.path)
                    .into_iter()
                    .map(|i| i + segment_offset),
            );
        }
        segment_offset += member_node.name.chars().count() as u32 + 1; // + '/'
    }
    match_indices.sort_unstable();
    match_indices.dedup();
    rows.push(Row {
        path: tail_node.path.clone(),
        name: label,
        kind: tail_node.kind,
        depth,
        expanded: tail_node.expanded,
        loaded: tail_node.is_loaded(),
        chain: if members.len() > 1 {
            members
                .iter()
                .map(|&m| ctx.tree.get(m).path.clone())
                .collect()
        } else {
            Vec::new()
        },
        status,
        ignored: ctx.is_ignored(&node.path),
        missing: false,
        search: ctx.search_flag(hit),
        match_indices,
        annotation: None,
    });
    if tail_node.kind.is_dir() && tail_node.expanded {
        push_children(ctx, tail, depth + 1, rows);
    }
}

/// The picker's filter render policy (ADR 0009): matches as a dense flat
/// list, best first, decorated from git state. The displayed string is the
/// relative path, so name-mode match indices shift by the dir-prefix length;
/// path-mode indices apply directly (ADR 0013).
pub fn match_rows(matches: &[Match], git: Option<&GitState>) -> Vec<Row> {
    matches
        .iter()
        .map(|m| {
            let entry = &m.entry;
            let kind = if entry.is_dir {
                NodeKind::Dir
            } else {
                NodeKind::File
            };
            let status = match git {
                Some(git) if entry.is_dir => git.dir_status(&entry.abs),
                Some(git) => git.status_of(&entry.abs),
                None => None,
            };
            let match_indices = if m.by_path {
                m.indices.clone()
            } else {
                m.indices.iter().map(|i| i + entry.name_offset).collect()
            };
            Row {
                path: entry.abs.clone(),
                name: entry.rel.clone(),
                kind,
                depth: 0,
                expanded: false,
                loaded: true,
                chain: Vec::new(),
                status,
                ignored: false,
                missing: false,
                search: None,
                match_indices,
                annotation: None,
            }
        })
        .collect()
}

/// `$HOME`-abbreviated display form of a path (IDEA-style root annotation).
pub fn abbreviate_home(path: &std::path::Path, home: Option<&std::path::Path>) -> String {
    if let Some(home) = home
        && let Ok(rest) = path.strip_prefix(home)
    {
        if rest.as_os_str().is_empty() {
            return "~".into();
        }
        return format!("~/{}", rest.display());
    }
    path.display().to_string()
}

#[derive(Default)]
pub struct FlatView {
    pub selection: Option<PathBuf>,
    pub scroll: usize,
    /// Chains split on demand (ADR 0014): members of these paths render as
    /// individual rows instead of compacting. Session-local; collapsing a
    /// dir prunes it and everything under it, re-fusing the chain.
    pub split: HashSet<PathBuf>,
    /// Remembered position so a vanished selection lands somewhere sensible.
    last_index: usize,
    /// Set when the selection moves; the next `reconcile` scrolls it into
    /// view. Wheel scrolling leaves it unset, so free scrolling is never
    /// snapped back.
    follow: bool,
}

impl FlatView {
    /// Reconciles selection with the current rows: keeps it if the path is
    /// still visible, otherwise falls back near the remembered position.
    /// Returns the selected index, if any rows exist.
    pub fn sync(&mut self, rows: &[Row]) -> Option<usize> {
        if rows.is_empty() {
            self.selection = None;
            return None;
        }
        if let Some(path) = &self.selection {
            if let Some(idx) = rows.iter().position(|r| &r.path == path) {
                self.last_index = idx;
                return Some(idx);
            }
            // A path that is an interior member of a compacted chain resolves
            // to the chain row (and normalizes the selection to its tail).
            if let Some(idx) = rows.iter().position(|r| r.chain.iter().any(|m| m == path)) {
                self.select(rows, idx);
                return Some(idx);
            }
        }
        let idx = self.last_index.min(rows.len() - 1);
        self.select(rows, idx);
        Some(idx)
    }

    /// Points the selection at a path (revealed by search or the socket) and
    /// scrolls it into view on the next reconcile.
    pub fn focus(&mut self, path: PathBuf) {
        self.selection = Some(path);
        self.follow = true;
    }

    fn select(&mut self, rows: &[Row], idx: usize) {
        self.selection = Some(rows[idx].path.clone());
        self.last_index = idx;
        self.follow = true;
    }

    pub fn move_by(&mut self, rows: &[Row], delta: isize) {
        let Some(idx) = self.sync(rows) else { return };
        let new = idx.saturating_add_signed(delta).min(rows.len() - 1);
        self.select(rows, new);
    }

    /// `→`: expand a dir (design doc keyboard table; collapse is `←`'s job).
    /// A chain expands at its tail; on an already-expanded chain, `→` splits
    /// it into its member rows (ADR 0014).
    pub fn on_right(&mut self, tree: &mut Tree, rows: &[Row]) -> NavEffect {
        let Some(idx) = self.sync(rows) else {
            return NavEffect::None;
        };
        let row = &rows[idx];
        if row.kind.is_dir() && !row.expanded && !row.missing {
            return self.expand(tree, row);
        }
        if row.kind.is_dir() && row.expanded && row.chain.len() > 1 {
            // Middles have exactly one visible child and are already loaded
            // (the chain could not have formed otherwise), so expanding them
            // is free — no I/O effect to request.
            for member in &row.chain {
                tree.set_expanded(member, true);
                self.split.insert(member.clone());
            }
        }
        NavEffect::None
    }

    /// `←`: collapse an expanded dir (a chain collapses back to one row);
    /// otherwise jump to the parent.
    pub fn on_left(&mut self, tree: &mut Tree, rows: &[Row]) {
        let Some(idx) = self.sync(rows) else { return };
        let row = &rows[idx];
        if row.kind.is_dir() && row.expanded {
            self.collapse(tree, row);
            return;
        }
        if let Some(parent_idx) = rows[..idx].iter().rposition(|r| r.depth + 1 == row.depth) {
            self.select(rows, parent_idx);
        }
    }

    /// Enter: toggle a dir, open a file. Enter always opens files — never
    /// contextual (design doc keyboard table).
    pub fn on_enter(&mut self, tree: &mut Tree, rows: &[Row]) -> NavEffect {
        let Some(idx) = self.sync(rows) else {
            return NavEffect::None;
        };
        self.activate(tree, rows, idx)
    }

    /// A single click on a row's name: selection only — activation is the
    /// double-click's job (ADR 0015).
    pub fn on_single_click(&mut self, rows: &[Row], idx: usize) {
        if idx < rows.len() {
            self.select(rows, idx);
        }
    }

    /// An activating click: a chevron click, or a completed double-click.
    /// Chevron clicks toggle without moving selection; double name clicks
    /// select, then open (file) or toggle (dir).
    pub fn on_click(
        &mut self,
        tree: &mut Tree,
        rows: &[Row],
        idx: usize,
        on_chevron: bool,
    ) -> NavEffect {
        let Some(row) = rows.get(idx) else {
            return NavEffect::None;
        };
        if row.kind.is_dir() && on_chevron && !row.missing {
            return self.toggle(tree, row);
        }
        self.select(rows, idx);
        if row.kind.is_dir() && !row.missing {
            self.toggle(tree, row)
        } else {
            self.activate(tree, rows, idx)
        }
    }

    fn activate(&mut self, tree: &mut Tree, rows: &[Row], idx: usize) -> NavEffect {
        let row = &rows[idx];
        if row.missing {
            return NavEffect::Message(format!("{} is deleted (tracked in git)", row.name));
        }
        if row.kind.is_dir() {
            return self.toggle(tree, row);
        }
        NavEffect::Open(row.path.clone())
    }

    fn toggle(&mut self, tree: &mut Tree, row: &Row) -> NavEffect {
        if row.expanded {
            self.collapse(tree, row);
            NavEffect::None
        } else {
            self.expand(tree, row)
        }
    }

    /// The one collapse path (arrow, Enter-toggle, mouse): collapsing a dir
    /// also prunes split-chain state under it, so the chain re-fuses
    /// (ADR 0014). Pruned members collapse with it — a fused chain row shows
    /// the tail's expansion, so a still-expanded tail would leak children
    /// under a collapsed head. Stale entries for vanished dirs are swept
    /// along the way.
    fn collapse(&mut self, tree: &mut Tree, row: &Row) {
        tree.set_expanded(&row.path, false);
        if !self.split.is_empty() {
            self.split.retain(|p| {
                let under = p.starts_with(&row.path);
                if under {
                    tree.set_expanded(p, false);
                }
                !under
            });
        }
    }

    fn expand(&mut self, tree: &mut Tree, row: &Row) -> NavEffect {
        tree.set_expanded(&row.path, true);
        if row.loaded {
            NavEffect::None
        } else {
            NavEffect::RequestExpand(row.path.clone())
        }
    }

    pub fn scroll_by(&mut self, rows: &[Row], delta: isize, viewport: usize) {
        let max_scroll = rows.len().saturating_sub(viewport);
        self.scroll = self.scroll.saturating_add_signed(delta).min(max_scroll);
    }

    /// Pre-draw reconciliation: re-resolves a vanished selection, clamps the
    /// scroll to the row count, and — only after the selection moved — scrolls
    /// it into view. Free wheel scrolling is never snapped back.
    pub fn reconcile(&mut self, rows: &[Row], viewport: usize) {
        let max_scroll = rows.len().saturating_sub(viewport);
        self.scroll = self.scroll.min(max_scroll);
        if viewport == 0 {
            return;
        }
        let Some(idx) = self.sync(rows) else { return };
        if !self.follow {
            return;
        }
        self.follow = false;
        if idx < self.scroll {
            self.scroll = idx;
        } else if idx >= self.scroll + viewport {
            self.scroll = idx + 1 - viewport;
        }
        self.scroll = self.scroll.min(max_scroll);
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use birch_core::git::parse_porcelain_v2;
    use birch_core::{Entry, TreeDelta};

    use super::*;

    fn entry(name: &str, kind: NodeKind) -> Entry {
        Entry {
            name: name.into(),
            kind,
        }
    }

    fn snapshot(tree: &mut Tree, dir: &str, entries: Vec<Entry>) {
        tree.apply(TreeDelta::Snapshot {
            dir: dir.into(),
            entries,
        });
    }

    /// Builds: root { src/ { main.rs, lib.rs }, .git/ { }, .env, readme.md }
    fn fixture() -> Tree {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.set_expanded(Path::new("/r"), true);
        snapshot(
            &mut tree,
            "/r",
            vec![
                entry("src", NodeKind::Dir),
                entry(".git", NodeKind::Dir),
                entry(".env", NodeKind::File),
                entry("readme.md", NodeKind::File),
            ],
        );
        snapshot(
            &mut tree,
            "/r/src",
            vec![
                entry("main.rs", NodeKind::File),
                entry("lib.rs", NodeKind::File),
            ],
        );
        tree
    }

    fn rows_plain(tree: &Tree) -> Vec<Row> {
        visible_rows(tree, &Settings::default(), Decor::default())
    }

    fn rows_with_git(tree: &Tree, s: &Settings, git: &GitState) -> Vec<Row> {
        visible_rows(
            tree,
            s,
            Decor {
                git: Some(git),
                ..Decor::default()
            },
        )
    }

    fn row_names(rows: &[Row]) -> Vec<&str> {
        rows.iter().map(|r| r.name.as_str()).collect()
    }

    fn git_fixture(records: &[&str]) -> GitState {
        let joined = records.join("\0") + "\0";
        parse_porcelain_v2(joined.as_bytes(), Path::new("/r"))
    }

    #[test]
    fn abbreviate_home_edges() {
        let home = Path::new("/Users/someone");
        assert_eq!(
            abbreviate_home(Path::new("/Users/someone/w/p"), Some(home)),
            "~/w/p"
        );
        assert_eq!(abbreviate_home(home, Some(home)), "~");
        assert_eq!(abbreviate_home(Path::new("/opt/x"), Some(home)), "/opt/x");
        assert_eq!(abbreviate_home(Path::new("/opt/x"), None), "/opt/x");
        // Component-wise: /Users/someone-else must not abbreviate.
        assert_eq!(
            abbreviate_home(Path::new("/Users/someone-else/x"), Some(home)),
            "/Users/someone-else/x"
        );
    }

    #[test]
    fn noise_hidden_by_default_dotfiles_shown() {
        let tree = fixture();
        assert_eq!(
            row_names(&rows_plain(&tree)),
            ["r", "src", ".env", "readme.md"]
        );
    }

    #[test]
    fn hide_hidden_hides_dotfiles() {
        let tree = fixture();
        let s = Settings {
            show_hidden: false,
            ..Settings::default()
        };
        let rows = visible_rows(&tree, &s, Decor::default());
        assert_eq!(row_names(&rows), ["r", "src", "readme.md"]);
    }

    #[test]
    fn show_noise_reveals_git_dir() {
        let tree = fixture();
        let s = Settings {
            show_noise: true,
            ..Settings::default()
        };
        let rows = visible_rows(&tree, &s, Decor::default());
        assert!(rows.iter().any(|r| r.name == ".git"));
    }

    #[test]
    fn expanded_dir_children_are_rows_with_depth() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let rows = rows_plain(&tree);
        assert_eq!(
            row_names(&rows),
            ["r", "src", "lib.rs", "main.rs", ".env", "readme.md"]
        );
        assert_eq!(rows[0].depth, 0);
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[2].depth, 2);
    }

    // ---- git decoration ----

    #[test]
    fn statuses_and_rollups_decorate_rows() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let git = git_fixture(&["1 .M N... 100644 100644 100644 a b src/main.rs"]);
        let rows = rows_with_git(&tree, &Settings::default(), &git);
        let by_name = |n: &str| rows.iter().find(|r| r.name == n).unwrap();
        assert_eq!(by_name("main.rs").status, Some(FileStatus::Modified));
        assert_eq!(by_name("src").status, Some(FileStatus::Modified)); // rollup
        assert_eq!(by_name("readme.md").status, None);
    }

    #[test]
    fn deleted_tracked_files_appear_as_missing_rows() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let git = git_fixture(&["1 .D N... 100644 100644 000000 a a src/gone.rs"]);
        let rows = rows_with_git(&tree, &Settings::default(), &git);
        let gone = rows.iter().find(|r| r.name == "gone.rs").unwrap();
        assert!(gone.missing);
        assert_eq!(gone.status, Some(FileStatus::Deleted));
        assert_eq!(gone.path, PathBuf::from("/r/src/gone.rs"));
        // Sorted among src's files, ahead of lib.rs and main.rs.
        assert_eq!(
            row_names(&rows),
            [
                "r",
                "src",
                "gone.rs",
                "lib.rs",
                "main.rs",
                ".env",
                "readme.md"
            ]
        );
    }

    #[test]
    fn ignored_rows_are_flagged_and_hideable() {
        let mut tree = fixture();
        snapshot(
            &mut tree,
            "/r",
            vec![
                entry("src", NodeKind::Dir),
                entry("target", NodeKind::Dir),
                entry(".env", NodeKind::File),
                entry("readme.md", NodeKind::File),
            ],
        );
        let git = git_fixture(&["! target/"]);
        let rows = rows_with_git(&tree, &Settings::default(), &git);
        assert!(rows.iter().find(|r| r.name == "target").unwrap().ignored);

        let s = Settings {
            show_ignored: false,
            ..Settings::default()
        };
        let rows = rows_with_git(&tree, &s, &git);
        assert!(!rows.iter().any(|r| r.name == "target"));
    }

    #[test]
    fn missing_row_activation_reports_instead_of_opening() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let git = git_fixture(&["1 .D N... 100644 100644 000000 a a src/gone.rs"]);
        let rows = rows_with_git(&tree, &Settings::default(), &git);
        let mut fv = FlatView {
            selection: Some("/r/src/gone.rs".into()),
            ..FlatView::default()
        };
        let effect = fv.on_enter(&mut tree, &rows);
        assert!(matches!(effect, NavEffect::Message(_)));
    }

    // ---- compaction ----

    /// root { a/b/c { file.txt }, other.txt }
    fn chain_fixture() -> Tree {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.set_expanded(Path::new("/r"), true);
        snapshot(
            &mut tree,
            "/r",
            vec![
                entry("a", NodeKind::Dir),
                entry("other.txt", NodeKind::File),
            ],
        );
        snapshot(&mut tree, "/r/a", vec![entry("b", NodeKind::Dir)]);
        snapshot(&mut tree, "/r/a/b", vec![entry("c", NodeKind::Dir)]);
        snapshot(
            &mut tree,
            "/r/a/b/c",
            vec![entry("file.txt", NodeKind::File)],
        );
        tree
    }

    #[test]
    fn single_click_selects_without_activating() {
        let mut tree = fixture();
        let mut view = FlatView::default();
        let rows = rows_plain(&tree);
        let dir_idx = rows.iter().position(|r| r.name == "src").unwrap();
        view.on_single_click(&rows, dir_idx);
        assert_eq!(view.selection, Some(PathBuf::from("/r/src")));
        // No toggle happened: the flattened tree is unchanged.
        assert_eq!(row_names(&rows_plain(&tree)), row_names(&rows));
        // The activating path (chevron or completed double-click) still
        // opens files.
        let file_idx = rows.iter().position(|r| r.name == "readme.md").unwrap();
        assert!(matches!(
            view.on_click(&mut tree, &rows, file_idx, false),
            NavEffect::Open(p) if p == Path::new("/r/readme.md")
        ));
    }

    #[test]
    fn single_child_chain_compacts_to_tail() {
        let tree = chain_fixture();
        let rows = rows_plain(&tree);
        assert_eq!(row_names(&rows), ["r", "a/b/c", "other.txt"]);
        let chain = &rows[1];
        assert_eq!(chain.path, PathBuf::from("/r/a/b/c"));
        assert_eq!(
            chain.chain,
            [
                PathBuf::from("/r/a"),
                PathBuf::from("/r/a/b"),
                PathBuf::from("/r/a/b/c")
            ]
        );
        assert!(!chain.expanded);
    }

    #[test]
    fn expanded_chain_shows_tail_children() {
        let mut tree = chain_fixture();
        tree.set_expanded(Path::new("/r/a/b/c"), true);
        let rows = rows_plain(&tree);
        assert_eq!(row_names(&rows), ["r", "a/b/c", "file.txt", "other.txt"]);
        assert_eq!(rows[2].depth, 2);
    }

    #[test]
    fn second_child_splits_the_chain() {
        let mut tree = chain_fixture();
        snapshot(
            &mut tree,
            "/r/a/b",
            vec![entry("c", NodeKind::Dir), entry("d.txt", NodeKind::File)],
        );
        let rows = rows_plain(&tree);
        // a/b still compacts; c no longer swallows into it.
        assert_eq!(row_names(&rows), ["r", "a/b", "other.txt"]);
        let ab = &rows[1];
        assert_eq!(ab.path, PathBuf::from("/r/a/b"));
    }

    #[test]
    fn unloaded_tail_renders_known_prefix() {
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.set_expanded(Path::new("/r"), true);
        snapshot(&mut tree, "/r", vec![entry("a", NodeKind::Dir)]);
        snapshot(&mut tree, "/r/a", vec![entry("b", NodeKind::Dir)]);
        // b never loaded: chain is a/b with an unloaded tail.
        let rows = rows_plain(&tree);
        assert_eq!(row_names(&rows), ["r", "a/b"]);
        assert!(!rows[1].loaded);
    }

    #[test]
    fn symlinked_dirs_never_join_chains() {
        // a's only child is a symlinked dir (worst case: a link back up the
        // tree) — the chain must stop at a, and the symlink stays its own row.
        let mut tree = Tree::new(PathBuf::from("/r"), false);
        tree.set_expanded(Path::new("/r"), true);
        snapshot(&mut tree, "/r", vec![entry("a", NodeKind::Dir)]);
        snapshot(&mut tree, "/r/a", vec![entry("loop", NodeKind::SymlinkDir)]);
        let rows = rows_plain(&tree);
        assert_eq!(row_names(&rows), ["r", "a"]);
        tree.set_expanded(Path::new("/r/a"), true);
        let rows = rows_plain(&tree);
        assert_eq!(row_names(&rows), ["r", "a", "loop"]);
        assert!(rows[2].chain.is_empty());
    }

    #[test]
    fn chains_stop_at_the_ignore_boundary() {
        let mut tree = chain_fixture();
        // b is gitignored: a stays alone, and expanding a shows b/c starting
        // its own (ignored) chain only from b — which stays uncompacted here
        // because the head itself is ignored.
        let git = git_fixture(&["! a/b/"]);
        let rows = rows_with_git(&tree, &Settings::default(), &git);
        let a = rows.iter().find(|r| r.name == "a").unwrap();
        assert!(a.chain.is_empty());

        tree.set_expanded(Path::new("/r/a"), true);
        let rows = rows_with_git(&tree, &Settings::default(), &git);
        let b = rows.iter().find(|r| r.name == "b").unwrap();
        assert!(b.chain.is_empty());
        assert!(b.ignored);
    }

    fn rows_split(tree: &Tree, view: &FlatView) -> Vec<Row> {
        visible_rows(
            tree,
            &Settings::default(),
            Decor {
                split: Some(&view.split),
                ..Decor::default()
            },
        )
    }

    /// A chain fixture with the tail expanded, split via `→` (ADR 0014).
    fn split_chain() -> (Tree, FlatView) {
        let mut tree = chain_fixture();
        tree.set_expanded(Path::new("/r/a/b/c"), true);
        let mut view = FlatView::default();
        view.focus(PathBuf::from("/r/a/b/c"));
        let rows = rows_split(&tree, &view);
        view.on_right(&mut tree, &rows);
        (tree, view)
    }

    #[test]
    fn right_on_expanded_chain_splits_it() {
        let (tree, view) = split_chain();
        let rows = rows_split(&tree, &view);
        assert_eq!(
            row_names(&rows),
            ["r", "a", "b", "c", "file.txt", "other.txt"]
        );
        // Members nest one level each, all expanded; selection stays on the
        // tail — nothing jumps.
        assert_eq!(rows[1].depth, 1);
        assert_eq!(rows[3].depth, 3);
        assert!(rows[1..4].iter().all(|r| r.expanded && r.chain.is_empty()));
        assert_eq!(view.selection.as_deref(), Some(Path::new("/r/a/b/c")));
    }

    #[test]
    fn right_on_expanded_plain_dir_still_does_nothing() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let mut view = FlatView::default();
        view.focus(PathBuf::from("/r/src"));
        let rows = rows_plain(&tree);
        view.on_right(&mut tree, &rows);
        assert!(view.split.is_empty());
        assert_eq!(row_names(&rows_plain(&tree)), row_names(&rows));
    }

    #[test]
    fn collapsing_the_head_refuses_the_chain() {
        let (mut tree, mut view) = split_chain();
        view.focus(PathBuf::from("/r/a"));
        let rows = rows_split(&tree, &view);
        view.on_left(&mut tree, &rows);
        let rows = rows_split(&tree, &view);
        assert_eq!(row_names(&rows), ["r", "a/b/c", "other.txt"]);
        assert!(!rows[1].expanded);
        assert!(view.split.is_empty());
    }

    #[test]
    fn collapsing_a_middle_refuses_only_the_subchain() {
        let (mut tree, mut view) = split_chain();
        view.focus(PathBuf::from("/r/a/b"));
        let rows = rows_split(&tree, &view);
        view.on_left(&mut tree, &rows);
        let rows = rows_split(&tree, &view);
        // The head stays split; b/c re-fuse below it as a collapsed chain.
        assert_eq!(row_names(&rows), ["r", "a", "b/c", "other.txt"]);
        assert!(rows[1].expanded);
        assert!(!rows[2].expanded);
    }

    #[test]
    fn split_survives_live_updates() {
        let (mut tree, view) = split_chain();
        // A watcher delta must not re-fuse the split chain — split state
        // lives in the view, not the tree.
        snapshot(
            &mut tree,
            "/r/a/b/c",
            vec![
                entry("file.txt", NodeKind::File),
                entry("new.txt", NodeKind::File),
            ],
        );
        let rows = rows_split(&tree, &view);
        assert_eq!(
            row_names(&rows),
            ["r", "a", "b", "c", "file.txt", "new.txt", "other.txt"]
        );
    }

    #[test]
    fn collapsing_root_prunes_splits_under_it() {
        let (mut tree, mut view) = split_chain();
        view.focus(PathBuf::from("/r"));
        let rows = rows_split(&tree, &view);
        view.on_left(&mut tree, &rows);
        assert!(view.split.is_empty());
        // Re-expanding shows the chain fused and collapsed again.
        tree.set_expanded(Path::new("/r"), true);
        let rows = rows_split(&tree, &view);
        assert_eq!(row_names(&rows), ["r", "a/b/c", "other.txt"]);
        assert!(!rows[1].expanded);
    }

    #[test]
    fn chain_labels_light_matched_members_at_their_offsets() {
        let tree = chain_fixture();
        // Name-mode hits on the head ("a", indices [0]) and the tail ("c",
        // indices [0]) of the a/b/c chain.
        let matched: HashMap<PathBuf, Vec<u32>> = [
            (PathBuf::from("/r/a"), vec![0]),
            (PathBuf::from("/r/a/b/c"), vec![0]),
        ]
        .into_iter()
        .collect();
        let rows = visible_rows(
            &tree,
            &Settings::default(),
            Decor {
                matched: Some(&matched),
                ..Decor::default()
            },
        );
        let chain = rows.iter().find(|r| r.name == "a/b/c").unwrap();
        assert_eq!(chain.search, Some(true));
        // Label chars: a(0) /(1) b(2) /(3) c(4).
        assert_eq!(chain.match_indices, [0, 4]);
    }

    #[test]
    fn picker_rows_shift_name_indices_onto_the_rel_path() {
        use birch_core::search::{IndexEntry, Match};
        let name_hit = Match {
            entry: IndexEntry::new("src/main.rs".into(), "/r/src/main.rs".into(), false),
            indices: vec![0, 1],
            by_path: false,
        };
        let path_hit = Match {
            entry: IndexEntry::new("src/lib.rs".into(), "/r/src/lib.rs".into(), false),
            indices: vec![0, 4],
            by_path: true,
        };
        let rows = match_rows(&[name_hit, path_hit], None);
        // "src/" is 4 chars: name indices 0,1 land at 4,5 of the rel path.
        assert_eq!(rows[0].match_indices, [4, 5]);
        // Path-mode indices already address the displayed rel path.
        assert_eq!(rows[1].match_indices, [0, 4]);
    }

    #[test]
    fn no_compact_disables_chains() {
        let tree = chain_fixture();
        let s = Settings {
            compact: false,
            ..Settings::default()
        };
        let rows = visible_rows(&tree, &s, Decor::default());
        assert_eq!(row_names(&rows), ["r", "a", "other.txt"]);
    }

    #[test]
    fn hidden_sibling_is_visibility_aware() {
        let mut tree = chain_fixture();
        // A dotfile sibling of b: chain holds with hidden files off, breaks
        // when they are shown.
        snapshot(
            &mut tree,
            "/r/a",
            vec![entry("b", NodeKind::Dir), entry(".hidden", NodeKind::File)],
        );
        let shown = rows_plain(&tree);
        assert_eq!(row_names(&shown), ["r", "a", "other.txt"]);
        let s = Settings {
            show_hidden: false,
            ..Settings::default()
        };
        let hidden = visible_rows(&tree, &s, Decor::default());
        assert_eq!(row_names(&hidden), ["r", "a/b/c", "other.txt"]);
    }

    #[test]
    fn chain_collapses_at_tail_with_left() {
        let mut tree = chain_fixture();
        tree.set_expanded(Path::new("/r/a/b/c"), true);
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/a/b/c".into());
        fv.on_left(&mut tree, &rows);
        assert!(!tree.node_at(Path::new("/r/a/b/c")).unwrap().expanded);
        let rows = rows_plain(&tree);
        assert_eq!(row_names(&rows), ["r", "a/b/c", "other.txt"]);
    }

    // ---- navigation & scrolling (sprint-001 behavior kept) ----

    #[test]
    fn right_expands_and_requests_unloaded() {
        let mut tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/src".into());
        assert_eq!(fv.on_right(&mut tree, &rows), NavEffect::None);
        assert!(tree.node_at(Path::new("/r/src")).unwrap().expanded);

        snapshot(
            &mut tree,
            "/r",
            vec![entry("src", NodeKind::Dir), entry("lazy", NodeKind::Dir)],
        );
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/lazy".into());
        assert_eq!(
            fv.on_right(&mut tree, &rows),
            NavEffect::RequestExpand("/r/lazy".into())
        );
    }

    #[test]
    fn left_collapses_then_jumps_to_parent() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/src/main.rs".into());
        fv.on_left(&mut tree, &rows);
        assert_eq!(fv.selection.as_deref(), Some(Path::new("/r/src")));

        let rows = rows_plain(&tree);
        fv.on_left(&mut tree, &rows);
        assert!(!tree.node_at(Path::new("/r/src")).unwrap().expanded);
    }

    #[test]
    fn enter_opens_files_and_toggles_dirs() {
        let mut tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/readme.md".into());
        assert_eq!(
            fv.on_enter(&mut tree, &rows),
            NavEffect::Open("/r/readme.md".into())
        );

        fv.selection = Some("/r/src".into());
        fv.on_enter(&mut tree, &rows);
        assert!(tree.node_at(Path::new("/r/src")).unwrap().expanded);
        // Enter on an expanded dir collapses it (toggle, VS Code-style).
        let rows = rows_plain(&tree);
        fv.on_enter(&mut tree, &rows);
        assert!(!tree.node_at(Path::new("/r/src")).unwrap().expanded);
    }

    #[test]
    fn chevron_click_toggles_without_moving_selection() {
        let mut tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/readme.md".into());
        fv.sync(&rows);
        let src_idx = rows.iter().position(|r| r.name == "src").unwrap();
        fv.on_click(&mut tree, &rows, src_idx, true);
        assert!(tree.node_at(Path::new("/r/src")).unwrap().expanded);
        assert_eq!(fv.selection.as_deref(), Some(Path::new("/r/readme.md")));
    }

    #[test]
    fn selection_is_stable_when_rows_appear_above() {
        let mut tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/readme.md".into());
        fv.sync(&rows);
        tree.set_expanded(Path::new("/r/src"), true);
        let rows = rows_plain(&tree);
        fv.sync(&rows);
        assert_eq!(fv.selection.as_deref(), Some(Path::new("/r/readme.md")));
    }

    #[test]
    fn vanished_selection_falls_back_near_position() {
        let mut tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/.env".into());
        fv.sync(&rows);
        tree.apply(TreeDelta::Removed {
            path: "/r/.env".into(),
        });
        let rows = rows_plain(&tree);
        fv.sync(&rows);
        assert_eq!(fv.selection.as_deref(), Some(Path::new("/r/readme.md")));
    }

    #[test]
    fn move_clamps_at_edges() {
        let tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.move_by(&rows, -5);
        assert_eq!(fv.selection.as_deref(), Some(Path::new("/r")));
        fv.move_by(&rows, 100);
        assert_eq!(fv.selection.as_deref(), Some(Path::new("/r/readme.md")));
    }

    #[test]
    fn reconcile_follows_moved_selection_minimally() {
        let tree = fixture();
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.move_by(&rows, 2);
        fv.reconcile(&rows, 2);
        assert_eq!(fv.scroll, 1);
        fv.move_by(&rows, -2);
        fv.reconcile(&rows, 2);
        assert_eq!(fv.scroll, 0);
    }

    #[test]
    fn wheel_scroll_is_not_snapped_back_to_selection() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.move_by(&rows, 0);
        fv.reconcile(&rows, 2);
        assert_eq!(fv.scroll, 0);

        fv.scroll_by(&rows, 3, 2);
        assert_eq!(fv.scroll, 3);
        fv.reconcile(&rows, 2);
        assert_eq!(fv.scroll, 3);

        fv.move_by(&rows, 1);
        fv.reconcile(&rows, 2);
        assert_eq!(fv.scroll, 1);
    }

    #[test]
    fn scroll_clamps_to_content_and_shrinking_rows() {
        let mut tree = fixture();
        tree.set_expanded(Path::new("/r/src"), true);
        let mut fv = FlatView::default();
        let rows = rows_plain(&tree);
        fv.selection = Some("/r/readme.md".into());
        fv.sync(&rows);
        fv.scroll_by(&rows, 100, 2);
        assert_eq!(fv.scroll, 4); // 6 rows - viewport 2

        tree.set_expanded(Path::new("/r/src"), false);
        let rows = rows_plain(&tree);
        fv.reconcile(&rows, 2);
        assert_eq!(fv.scroll, 2); // 4 rows - viewport 2
    }
}
