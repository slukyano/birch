//! Fuzzy filename search (ADRs 0009/0013): an index worker walks the root
//! with the `ignore` crate; `search` scores the query against simple names —
//! or full relative paths when the query contains `/` — with nucleo, and
//! reports the matched character positions. One engine, one behaviour: the
//! pane keeps its tree and steps over the matches, in the picker too (ADR 0023).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use unicode_segmentation::UnicodeSegmentation;

use crate::settings;

#[derive(Clone, Debug)]
pub struct IndexEntry {
    /// Root-relative path, `/`-separated (what the picker displays).
    pub rel: String,
    pub abs: PathBuf,
    pub is_dir: bool,
    /// The simple name — the default match corpus (ADR 0013).
    pub name: String,
    /// Char offset of `name` within `rel` (maps name-mode match indices onto
    /// the displayed relative path).
    pub name_offset: u32,
}

impl IndexEntry {
    pub fn new(rel: String, abs: PathBuf, is_dir: bool) -> Self {
        let name = rel.rsplit('/').next().unwrap_or(&rel).to_string();
        let name_offset = (rel.chars().count() - name.chars().count()) as u32;
        Self {
            rel,
            abs,
            is_dir,
            name,
            name_offset,
        }
    }
}

/// One search hit: the entry plus the matched character positions — into
/// `name` for name-mode queries, into `rel` when the query contained `/`.
#[derive(Clone, Debug)]
pub struct Match {
    pub entry: IndexEntry,
    pub indices: Vec<u32>,
    pub by_path: bool,
}

/// A node stores only what is not derivable: which entry holds it, and where
/// its own name lives in the chunk's slab. Relative and absolute paths are
/// reconstructed by walking parents (ADR 0026) — storing them per entry cost
/// ten times this on a home-sized root.
#[derive(Clone, Copy, Debug)]
struct Node {
    /// Global index of the parent entry; `NO_PARENT` for a child of the root.
    parent: u32,
    name_start: u32,
    name_len: u32,
    is_dir: bool,
}

const NO_PARENT: u32 = u32::MAX;

/// A published slice of the index. Immutable once built, so the walk can keep
/// growing the index without rewriting what search is already reading.
#[derive(Debug)]
struct Chunk {
    /// Basenames, concatenated; nodes slice into this.
    names: String,
    nodes: Vec<Node>,
    /// Global index of `nodes[0]`.
    start: u32,
}

/// The search corpus. Chunked and append-only: a snapshot shares its chunks
/// with the builder rather than copying them, so publishing while the walk
/// runs is cheap and a snapshot never changes underfoot (ADR 0026).
#[derive(Default, Debug, Clone)]
pub struct SearchIndex {
    root: PathBuf,
    chunks: Vec<Arc<Chunk>>,
    len: u32,
    complete: bool,
}

impl SearchIndex {
    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Whether the walk that produced these chunks ran to the end. A partial
    /// index is searchable; it simply cannot support "no matches" as a claim.
    pub fn is_complete(&self) -> bool {
        self.complete
    }

    /// The root this index was built for. A walk in flight when the root
    /// changes keeps publishing; the consumer checks this before adopting a
    /// snapshot, so a complete index for the old corpus is never mistaken for
    /// one describing the new.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn node(&self, global: u32) -> Option<(&Chunk, &Node)> {
        let chunk = self.chunks.get(
            self.chunks
                .partition_point(|c| c.start <= global)
                .checked_sub(1)?,
        )?;
        let node = chunk.nodes.get((global - chunk.start) as usize)?;
        Some((chunk, node))
    }

    fn name_in<'a>(chunk: &'a Chunk, node: &Node) -> &'a str {
        &chunk.names[node.name_start as usize..(node.name_start + node.name_len) as usize]
    }

    /// Writes the root-relative path of `global` into `buf`, which is cleared
    /// first. One reused buffer serves a whole scan — nothing is materialised
    /// per entry.
    fn rel_into(&self, global: u32, buf: &mut String) {
        buf.clear();
        let mut parts: Vec<&str> = Vec::new();
        let mut cursor = Some(global);
        while let Some(idx) = cursor {
            let Some((chunk, node)) = self.node(idx) else {
                break;
            };
            parts.push(Self::name_in(chunk, node));
            cursor = (node.parent != NO_PARENT).then_some(node.parent);
        }
        for (i, part) in parts.iter().rev().enumerate() {
            if i > 0 {
                buf.push('/');
            }
            buf.push_str(part);
        }
    }

    /// The materialised entry for one node — built for search hits only.
    fn entry_at(&self, global: u32, buf: &mut String) -> Option<IndexEntry> {
        let (_, node) = self.node(global)?;
        self.rel_into(global, buf);
        Some(IndexEntry::new(
            buf.clone(),
            self.root.join(&*buf),
            node.is_dir,
        ))
    }

    /// Every entry's root-relative path. Materialises the whole corpus, so it
    /// is for tests and diagnostics — search never calls it.
    pub fn rel_paths(&self) -> Vec<String> {
        let mut buf = String::new();
        (0..self.len)
            .map(|global| {
                self.rel_into(global, &mut buf);
                buf.clone()
            })
            .collect()
    }

    /// As [`SearchIndex::from_paths`], but marked incomplete — the shape a
    /// mid-walk snapshot has.
    pub fn from_paths_partial(root: &Path, paths: &[(&str, bool)]) -> Self {
        let mut index = Self::from_paths(root, paths);
        index.complete = false;
        index
    }

    /// Builds an index from root-relative paths, creating ancestors as needed.
    /// For tests and for callers that already know the corpus.
    pub fn from_paths(root: &Path, paths: &[(&str, bool)]) -> Self {
        let mut builder = IndexBuilder::new(root.to_path_buf());
        let mut seen: HashMap<String, u32> = HashMap::new();
        for (rel, is_dir) in paths {
            let mut parent = NO_PARENT;
            let mut prefix = String::new();
            let count = rel.split('/').count();
            for (i, part) in rel.split('/').enumerate() {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(part);
                let last = i + 1 == count;
                if let Some(&existing) = seen.get(&prefix) {
                    parent = existing;
                    continue;
                }
                let idx = builder.push(parent, part, if last { *is_dir } else { true });
                seen.insert(prefix.clone(), idx);
                parent = idx;
            }
        }
        builder.finish()
    }
}

/// Accumulates nodes into chunks, handing out immutable snapshots as it goes.
struct IndexBuilder {
    root: PathBuf,
    chunks: Vec<Arc<Chunk>>,
    names: String,
    nodes: Vec<Node>,
    start: u32,
    chunk_size: usize,
}

/// Entries per published chunk. Small enough that the first results appear
/// almost immediately, large enough that a home-sized root publishes ~100
/// times rather than thousands.
const CHUNK_ENTRIES: usize = 8192;

impl IndexBuilder {
    fn new(root: PathBuf) -> Self {
        Self::with_chunk_size(root, CHUNK_ENTRIES)
    }

    fn with_chunk_size(root: PathBuf, chunk_size: usize) -> Self {
        Self {
            root,
            chunks: Vec::new(),
            names: String::new(),
            nodes: Vec::new(),
            start: 0,
            chunk_size,
        }
    }

    fn push(&mut self, parent: u32, name: &str, is_dir: bool) -> u32 {
        let node = Node {
            parent,
            name_start: self.names.len() as u32,
            name_len: name.len() as u32,
            is_dir,
        };
        self.names.push_str(name);
        self.nodes.push(node);
        self.start + self.nodes.len() as u32 - 1
    }

    fn full(&self) -> bool {
        self.nodes.len() >= self.chunk_size
    }

    /// Seals the open chunk. A no-op when nothing has accumulated.
    fn seal(&mut self) {
        if self.nodes.is_empty() {
            return;
        }
        let nodes = std::mem::take(&mut self.nodes);
        let names = std::mem::take(&mut self.names);
        let start = self.start;
        self.start += nodes.len() as u32;
        self.chunks.push(Arc::new(Chunk {
            names,
            nodes,
            start,
        }));
    }

    /// A snapshot of everything sealed so far. Cheap: it clones a vector of
    /// `Arc`s, never the entries themselves.
    fn snapshot(&self, complete: bool) -> SearchIndex {
        SearchIndex {
            root: self.root.clone(),
            chunks: self.chunks.clone(),
            len: self.start,
            complete,
        }
    }

    fn finish(mut self) -> SearchIndex {
        self.seal();
        self.snapshot(true)
    }
}

/// Walks the root, publishing the index as it grows: gitignore-aware, never
/// descending into ignored dirs, always skipping noise, honoring the
/// hidden-files setting. `publish` is called with each snapshot, and returning
/// `false` from it abandons the walk (the root changed, or birch is exiting).
pub fn build_index_streaming(
    root: &Path,
    show_hidden: bool,
    mut publish: impl FnMut(SearchIndex) -> bool,
) -> SearchIndex {
    let mut builder = IndexBuilder::new(root.to_path_buf());
    // The serial walker is depth-first and yields a parent before its
    // children, so the most recent entry one level up *is* the parent.
    let mut last_at_depth: Vec<u32> = Vec::new();
    let walker = ignore::WalkBuilder::new(root)
        .hidden(!show_hidden)
        .filter_entry(|entry| {
            entry
                .file_name()
                .to_str()
                .is_none_or(|name| !settings::is_noise(name))
        })
        .build();
    for entry in walker.flatten() {
        let abs = entry.path();
        let depth = entry.depth();
        if depth == 0 || abs == root {
            continue;
        }
        // Every entry must be recorded, even one whose name is not UTF-8:
        // skipping it would leave its children pointing at the previous
        // sibling, naming paths that do not exist.
        let name = abs
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The walker does not follow links, so a symlinked directory reports
        // as a symlink here while the tree calls it a directory. Resolve it, or
        // tree ordering, the forward anchor, and the filter each disagree with
        // the row on screen.
        let file_type = entry.file_type();
        let is_dir = file_type.is_some_and(|t| t.is_dir())
            || (file_type.is_some_and(|t| t.is_symlink()) && abs.is_dir());
        let parent = if depth <= 1 {
            NO_PARENT
        } else {
            last_at_depth.get(depth - 2).copied().unwrap_or(NO_PARENT)
        };
        let idx = builder.push(parent, &name, is_dir);
        if last_at_depth.len() < depth {
            last_at_depth.resize(depth, NO_PARENT);
        }
        last_at_depth[depth - 1] = idx;
        if builder.full() {
            builder.seal();
            if !publish(builder.snapshot(false)) {
                return builder.snapshot(false);
            }
        }
    }
    builder.finish()
}

/// Builds the index in one go, without publishing intermediate snapshots.
pub fn build_index(root: &Path, show_hidden: bool) -> SearchIndex {
    build_index_streaming(root, show_hidden, |_| true)
}

/// Scores `query` against the index, best first, with matched character
/// positions. Name corpus by default; a `/` in the query switches to full
/// relative paths (ADR 0013). Smart-case gives camel-hump anchoring (an
/// uppercase query char only matches capitals).
pub fn search(index: &SearchIndex, query: &str) -> Vec<Match> {
    if query.is_empty() {
        return Vec::new();
    }
    let by_path = query.contains('/');
    let mut matcher = if by_path {
        Matcher::new(Config::DEFAULT.match_paths())
    } else {
        Matcher::new(Config::DEFAULT)
    };
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    // One reused path buffer for the whole scan: under a path query every
    // candidate is reconstructed into it and discarded, and only hits are
    // materialised into an `IndexEntry` (ADR 0026).
    let mut path = String::new();
    let mut scored: Vec<(u32, Match)> = Vec::new();
    for global in 0..index.len {
        let Some((chunk, node)) = index.node(global) else {
            continue;
        };
        let haystack: &str = if by_path {
            index.rel_into(global, &mut path);
            &path
        } else {
            SearchIndex::name_in(chunk, node)
        };
        let mut indices = Vec::new();
        let Some(score) = pattern.indices(
            Utf32Str::new(haystack, &mut buf),
            &mut matcher,
            &mut indices,
        ) else {
            continue;
        };
        indices.sort_unstable();
        indices.dedup();
        let indices = to_char_indices(haystack, &indices);
        let mut entry_buf = String::new();
        let Some(entry) = index.entry_at(global, &mut entry_buf) else {
            continue;
        };
        scored.push((
            score,
            Match {
                entry,
                indices,
                by_path,
            },
        ));
    }
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score)); // stable: ties keep index order
    scored.into_iter().map(|(_, m)| m).collect()
}

/// Sorts matches into tree display order — the order `visible_rows` draws them
/// in, so stepping over them always travels visually down or up (task 063).
/// Ranking stays `search`'s job; this is the order the *pane* has.
pub fn sort_tree_order(matches: &mut [Match]) {
    matches.sort_by_cached_key(|m| tree_key(&m.entry.rel, m.entry.is_dir));
}

/// Index of the first match at or after `(rel, is_dir)` in tree order — the
/// insertion point, so it is `matches.len()` when every match precedes it.
pub fn tree_order_position(matches: &[Match], rel: &str, is_dir: bool) -> usize {
    let key = tree_key(rel, is_dir);
    matches.partition_point(|m| tree_key(&m.entry.rel, m.entry.is_dir) < key)
}

/// The sort key that reproduces the pane's row order for a root-relative path:
/// one entry per component, each `(is_file, lowercased name)`. Every component
/// but the last names a directory, and the last takes the entry's own kind — so
/// `false < true` puts directories ahead of files at the level where two paths
/// diverge, exactly as `visible_children` sorts them. A shorter key sorts first,
/// which is where the tree draws an ancestor.
fn tree_key(rel: &str, is_dir: bool) -> Vec<(bool, String)> {
    let count = rel.split('/').count();
    rel.split('/')
        .enumerate()
        .map(|(i, part)| (i + 1 == count && !is_dir, part.to_lowercase()))
        .collect()
}

/// Maps nucleo match positions onto char indices of the original string.
/// `Utf32Str::new`'s segmentation (verified against nucleo-matcher's source)
/// is not always char-based:
/// - an ASCII haystack matches over bytes — bytes == chars, identity;
/// - a non-ASCII haystack whose graphemes *reduce* to ASCII (e.g. NFD
///   `cafe\u{301}.txt`, the normal form macOS produces) matches over the raw
///   bytes of the original string — positions are byte offsets;
/// - anything else matches over one unit per grapheme cluster.
fn to_char_indices(haystack: &str, units: &[u32]) -> Vec<u32> {
    if haystack.is_ascii() {
        return units.to_vec();
    }
    let reduced_is_ascii = haystack
        .graphemes(true)
        .all(|g| g.chars().next().is_some_and(|c| c.is_ascii()));
    if reduced_is_ascii {
        // Byte offset → char index of the char containing that byte.
        let mut char_at_byte = vec![0u32; haystack.len()];
        for (char_idx, (byte_idx, ch)) in haystack.char_indices().enumerate() {
            char_at_byte[byte_idx..byte_idx + ch.len_utf8()].fill(char_idx as u32);
        }
        let mut out: Vec<u32> = units
            .iter()
            .filter_map(|&u| char_at_byte.get(u as usize).copied())
            .collect();
        out.dedup();
        return out;
    }
    // Grapheme index → char index of the grapheme's first char.
    let mut starts = Vec::new();
    let mut char_idx = 0u32;
    for grapheme in haystack.graphemes(true) {
        starts.push(char_idx);
        char_idx += grapheme.chars().count() as u32;
    }
    units
        .iter()
        .filter_map(|&u| starts.get(u as usize).copied())
        .collect()
}

#[derive(Clone, Debug)]
pub enum IndexCmd {
    /// Rebuild for the given root with the given hidden-files setting — both
    /// can change at runtime (`birch-ctl set-root` / `set hidden`).
    Rebuild { root: PathBuf, show_hidden: bool },
}

#[derive(Clone, Debug)]
pub enum IndexEvent {
    Index(Arc<SearchIndex>),
}

/// Rebuilds younger than this are skipped (dirty batches arrive in bursts).
pub const REBUILD_THROTTLE: Duration = Duration::from_millis(500);

pub struct IndexWorker;

impl IndexWorker {
    pub fn spawn(commands: Receiver<IndexCmd>, events: Sender<IndexEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("search-index".into())
            .spawn(move || {
                let mut last_build: Option<Instant> = None;
                loop {
                    let (mut root, mut show_hidden) = match commands.recv() {
                        Ok(IndexCmd::Rebuild { root, show_hidden }) => (root, show_hidden),
                        Err(_) => return,
                    };
                    let collapse = |root: &mut PathBuf, show_hidden: &mut bool| {
                        while let Ok(IndexCmd::Rebuild {
                            root: r,
                            show_hidden: s,
                        }) = commands.try_recv()
                        {
                            *root = r;
                            *show_hidden = s;
                        }
                    };
                    collapse(&mut root, &mut show_hidden);
                    if let Some(last) = last_build {
                        // Read elapsed once: re-reading after the comparison
                        // could cross the threshold and underflow.
                        let elapsed = last.elapsed();
                        if elapsed < REBUILD_THROTTLE {
                            thread::sleep(REBUILD_THROTTLE - elapsed);
                            collapse(&mut root, &mut show_hidden);
                        }
                    }
                    // Publish as the walk grows the index (ADR 0026). The walk
                    // always runs to the end: abandoning it for a newer request
                    // throws away every chunk already published and restarts at
                    // zero, so under filesystem churn the index would never
                    // reach `complete` and "no matches" could never be said.
                    // A queued rebuild simply runs next.
                    let index = build_index_streaming(&root, show_hidden, |snapshot| {
                        events.send(IndexEvent::Index(Arc::new(snapshot))).is_ok()
                    });
                    last_build = Some(Instant::now());
                    if events.send(IndexEvent::Index(Arc::new(index))).is_err() {
                        return;
                    }
                }
            })
            .expect("spawn search-index thread")
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn fixture_root(tag: &str) -> PathBuf {
        let tmp = std::env::temp_dir().join(format!("birch-search-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::create_dir_all(tmp.join("target/debug")).unwrap();
        fs::create_dir_all(tmp.join(".git")).unwrap();
        fs::write(tmp.join(".gitignore"), "target/\n").unwrap();
        fs::write(tmp.join("src/main.rs"), "x").unwrap();
        fs::write(tmp.join("src/lib.rs"), "x").unwrap();
        fs::write(tmp.join("target/debug/out.o"), "x").unwrap();
        fs::write(tmp.join(".env"), "x").unwrap();
        tmp.canonicalize().unwrap()
    }

    #[test]
    fn index_skips_ignored_noise_and_honors_hidden() {
        let root = fixture_root("index");
        let index = build_index(&root, true);
        let rels: Vec<String> = index.rel_paths();
        assert!(rels.iter().any(|r| r == "src/main.rs"));
        assert!(
            rels.iter().any(|r| r == ".env"),
            "hidden shown when enabled: {rels:?}"
        );
        assert!(rels.iter().any(|r| r == ".gitignore"));
        assert!(
            !rels.iter().any(|r| r.starts_with("target")),
            "ignored dirs are never descended into: {rels:?}"
        );
        assert!(!rels.iter().any(|r| r.starts_with(".git/") || r == ".git"));

        let index = build_index(&root, false);
        let rels: Vec<String> = index.rel_paths();
        assert!(
            !rels.iter().any(|r| r == ".env"),
            "hidden skipped when disabled"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    // ---- ADR 0026: a tree-shaped index, published while it is built ----

    #[test]
    fn paths_are_reconstructed_by_walking_parents() {
        // Nothing stores a path: each node knows its parent and its own name.
        let index = SearchIndex::from_paths(
            Path::new("/r"),
            &[("a/b/c/deep.rs", false), ("a/other.rs", false)],
        );
        let rels = index.rel_paths();
        assert!(rels.iter().any(|r| r == "a/b/c/deep.rs"));
        assert!(rels.iter().any(|r| r == "a/other.rs"));
        // Ancestors exist as entries in their own right.
        assert!(rels.iter().any(|r| r == "a"));
        assert!(rels.iter().any(|r| r == "a/b"));
        assert!(rels.iter().any(|r| r == "a/b/c"));
        // And a hit carries the absolute path, derived from the root.
        let hit = &search(&index, "deep")[0];
        assert_eq!(hit.entry.rel, "a/b/c/deep.rs");
        assert_eq!(hit.entry.abs, PathBuf::from("/r/a/b/c/deep.rs"));
        assert_eq!(hit.entry.name, "deep.rs");
    }

    #[test]
    fn a_parent_in_an_earlier_chunk_still_resolves() {
        // The chunk boundary is invisible to lookup: a node's parent may live
        // in a chunk sealed long before it.
        let mut builder = IndexBuilder::with_chunk_size(PathBuf::from("/r"), 2);
        let a = builder.push(NO_PARENT, "a", true);
        let b = builder.push(a, "b", true); // seals after this pair
        builder.seal();
        let c = builder.push(b, "c", true);
        let deep = builder.push(c, "deep.rs", false);
        let index = builder.finish();
        assert_eq!(index.len(), 4);
        let mut buf = String::new();
        index.rel_into(deep, &mut buf);
        assert_eq!(buf, "a/b/c/deep.rs");
        // And search spans chunks.
        assert_eq!(search(&index, "deep")[0].entry.rel, "a/b/c/deep.rs");
    }

    #[test]
    fn snapshots_are_searchable_before_the_walk_completes() {
        let mut builder = IndexBuilder::with_chunk_size(PathBuf::from("/r"), 2);
        builder.push(NO_PARENT, "early.rs", false);
        builder.push(NO_PARENT, "second.rs", false);
        builder.seal();
        let partial = builder.snapshot(false);
        assert!(!partial.is_complete(), "a growing index is not complete");
        assert_eq!(partial.len(), 2);
        assert_eq!(search(&partial, "early")[0].entry.rel, "early.rs");

        builder.push(NO_PARENT, "late.rs", false);
        let full = builder.finish();
        assert!(full.is_complete());
        assert_eq!(full.len(), 3);
        // The earlier snapshot is untouched by the growth.
        assert_eq!(partial.len(), 2);
        assert!(search(&partial, "late").is_empty());
        assert_eq!(search(&full, "late")[0].entry.rel, "late.rs");
    }

    #[test]
    fn a_streaming_walk_publishes_and_finishes_complete() {
        let root = std::env::temp_dir().join(format!("birch-stream-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("sub")).unwrap();
        for i in 0..5 {
            fs::write(root.join("sub").join(format!("f{i}.txt")), "").unwrap();
        }
        let mut seen = 0;
        let index = build_index_streaming(&root, true, |snapshot| {
            assert!(!snapshot.is_complete(), "mid-walk snapshots are partial");
            seen += 1;
            true
        });
        assert!(index.is_complete());
        let rels = index.rel_paths();
        assert!(rels.iter().any(|r| r == "sub/f0.txt"));
        // Too few entries to fill a chunk, so nothing was published early.
        assert_eq!(seen, 0);
        fs::remove_dir_all(&root).unwrap();
    }

    fn fixture_index() -> SearchIndex {
        SearchIndex::from_paths(
            Path::new("/r"),
            &[
                ("src/main.rs", false),
                ("src", true),
                ("docs/manual.md", false),
                ("lib/FooBar.ts", false),
            ],
        )
    }

    /// Builds matches for arbitrary `(rel, is_dir)` pairs — the ordering
    /// helpers only read the entry, never the score or the match positions.
    fn matches_of(entries: &[(&str, bool)]) -> Vec<Match> {
        entries
            .iter()
            .map(|(rel, is_dir)| Match {
                entry: IndexEntry::new((*rel).into(), PathBuf::from("/r").join(rel), *is_dir),
                indices: Vec::new(),
                by_path: false,
            })
            .collect()
    }

    #[test]
    fn tree_order_matches_how_rows_are_drawn() {
        // Deliberately scrambled: score order bears no relation to row order.
        let mut hits = matches_of(&[
            ("src/main.rs", false),
            ("Zeta.md", false),
            ("src", true),
            ("docs/api.md", false),
            ("src/cli", true),
            ("alpha.md", false),
            ("src/cli/args.rs", false),
            ("docs", true),
        ]);
        sort_tree_order(&mut hits);
        let order: Vec<&str> = hits.iter().map(|m| m.entry.rel.as_str()).collect();
        assert_eq!(
            order,
            [
                // Directories precede files at each level, names compare
                // case-insensitively, and a directory precedes its contents.
                "docs",
                "docs/api.md",
                "src",
                "src/cli",
                "src/cli/args.rs",
                "src/main.rs",
                "alpha.md",
                "Zeta.md",
            ]
        );
    }

    #[test]
    fn tree_order_position_is_the_forward_anchor() {
        let mut hits = matches_of(&[
            ("docs/api.md", false),
            ("src", true),
            ("src/main.rs", false),
            ("zeta.md", false),
        ]);
        sort_tree_order(&mut hits);

        // Before every match.
        assert_eq!(tree_order_position(&hits, "aaa", true), 0);
        // Exactly on a match: the anchor is that match, so it does not move.
        assert_eq!(tree_order_position(&hits, "src", true), 1);
        // Between two matches: the following one.
        assert_eq!(tree_order_position(&hits, "src/lib.rs", false), 2);
        // A top-level file anchors after every top-level directory subtree,
        // because directories sort first at each level, then by name.
        assert_eq!(tree_order_position(&hits, "alpha.md", false), 3);
        // Past the last match: the caller wraps this to the first.
        assert_eq!(tree_order_position(&hits, "zzz.txt", false), hits.len());
    }

    #[test]
    fn search_scores_and_filters() {
        let index = fixture_index();
        let hits = search(&index, "man");
        assert!(hits.iter().any(|m| m.entry.rel == "src/main.rs"));
        assert!(hits.iter().any(|m| m.entry.rel == "docs/manual.md"));
        // An exact-name query has a single, unambiguous winner.
        let hits = search(&index, "main.rs");
        assert_eq!(hits[0].entry.rel, "src/main.rs");

        assert!(search(&index, "").is_empty());
        assert!(search(&index, "zzzz").is_empty());
    }

    #[test]
    fn names_are_the_default_corpus_and_slash_switches_to_paths() {
        let index = fixture_index();
        // "sm" would match s(rc)/m(ain) only through the invisible path —
        // name-mode must not produce it (ADR 0013).
        assert!(
            search(&index, "sm")
                .iter()
                .all(|m| m.entry.rel != "src/main.rs"),
            "no cross-segment matches without a slash"
        );
        // With a slash the same intent works, flagged as a path match.
        let hits = search(&index, "s/m");
        let hit = hits
            .iter()
            .find(|m| m.entry.rel == "src/main.rs")
            .expect("path mode reaches it");
        assert!(hit.by_path);
    }

    #[test]
    fn match_indices_point_into_the_displayed_name() {
        let index = fixture_index();
        let hits = search(&index, "man");
        let hit = hits
            .iter()
            .find(|m| m.entry.rel == "docs/manual.md")
            .unwrap();
        assert!(!hit.by_path);
        // "man" lights the first three chars of "manual.md".
        assert_eq!(hit.indices, [0, 1, 2]);
        assert_eq!(hit.entry.name, "manual.md");
        assert_eq!(hit.entry.name_offset, 5); // after "docs/"
    }

    #[test]
    fn nfd_names_map_match_positions_to_char_indices() {
        // macOS filesystems store accented names in NFD; nucleo matches such
        // strings over raw bytes, so the mapping back to chars is load-
        // bearing for highlighting.
        let index = SearchIndex::from_paths(Path::new("/r"), &[("cafe\u{301}.txt", false)]);
        let hits = search(&index, "txt");
        let hit = &hits[0];
        // Chars: c(0) a(1) f(2) e(3) combining(4) .(5) t(6) x(7) t(8).
        assert_eq!(hit.indices, [6, 7, 8]);

        // Fully non-ASCII graphemes go through the grapheme mapping.
        let index = SearchIndex::from_paths(Path::new("/r"), &[("день.txt", false)]);
        let hits = search(&index, "txt");
        assert_eq!(hits[0].indices, [5, 6, 7]); // д(0)е(1)н(2)ь(3).(4)t(5)x(6)t(7)
    }

    #[test]
    fn smart_case_matches_and_camel_humps_anchor() {
        let index = fixture_index();
        assert!(
            !search(&index, "foobar").is_empty(),
            "an all-lowercase query matches case-insensitively"
        );
        // Uppercase anchors to capitals: FB finds FooBar at F and B.
        let hits = search(&index, "FB");
        let hit = hits.iter().find(|m| m.entry.name == "FooBar.ts").unwrap();
        assert_eq!(hit.indices, [0, 3]);
    }
}
