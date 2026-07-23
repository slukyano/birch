//! Fuzzy filename search (ADRs 0009/0013): an index worker walks the root
//! with the `ignore` crate; `search` scores the query against simple names —
//! or full relative paths when the query contains `/` — with nucleo, and
//! reports the matched character positions. One engine — the main pane jumps
//! over the matches, the picker filters to them.

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

#[derive(Default, Debug)]
pub struct SearchIndex {
    pub entries: Vec<IndexEntry>,
}

/// Builds the index synchronously: gitignore-aware, never descending into
/// ignored dirs, always skipping noise, honoring the hidden-files setting.
pub fn build_index(root: &Path, show_hidden: bool) -> SearchIndex {
    let mut entries = Vec::new();
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
        if abs == root {
            continue;
        }
        let Ok(rel_path) = abs.strip_prefix(root) else {
            continue;
        };
        let rel = rel_path.to_string_lossy().replace('\\', "/");
        let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
        entries.push(IndexEntry::new(rel, abs.to_path_buf(), is_dir));
    }
    SearchIndex { entries }
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
    let mut scored: Vec<(u32, Match)> = index
        .entries
        .iter()
        .filter_map(|entry| {
            let haystack = if by_path { &entry.rel } else { &entry.name };
            let mut indices = Vec::new();
            let score = pattern.indices(
                Utf32Str::new(haystack, &mut buf),
                &mut matcher,
                &mut indices,
            )?;
            indices.sort_unstable();
            indices.dedup();
            let indices = to_char_indices(haystack, &indices);
            Some((
                score,
                Match {
                    entry: entry.clone(),
                    indices,
                    by_path,
                },
            ))
        })
        .collect();
    scored.sort_by_key(|(score, _)| std::cmp::Reverse(*score)); // stable: ties keep index order
    scored.into_iter().map(|(_, m)| m).collect()
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
                while let Ok(IndexCmd::Rebuild {
                    mut root,
                    mut show_hidden,
                }) = commands.recv()
                {
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
                    let index = Arc::new(build_index(&root, show_hidden));
                    last_build = Some(Instant::now());
                    if events.send(IndexEvent::Index(index)).is_err() {
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
        let rels: Vec<&str> = index.entries.iter().map(|e| e.rel.as_str()).collect();
        assert!(rels.contains(&"src/main.rs"));
        assert!(
            rels.contains(&".env"),
            "hidden shown when enabled: {rels:?}"
        );
        assert!(rels.contains(&".gitignore"));
        assert!(
            !rels.iter().any(|r| r.starts_with("target")),
            "ignored dirs are never descended into: {rels:?}"
        );
        assert!(!rels.iter().any(|r| r.starts_with(".git/") || *r == ".git"));

        let index = build_index(&root, false);
        let rels: Vec<&str> = index.entries.iter().map(|e| e.rel.as_str()).collect();
        assert!(!rels.contains(&".env"), "hidden skipped when disabled");
        fs::remove_dir_all(&root).unwrap();
    }

    fn fixture_index() -> SearchIndex {
        SearchIndex {
            entries: vec![
                IndexEntry::new("src/main.rs".into(), "/r/src/main.rs".into(), false),
                IndexEntry::new("src".into(), "/r/src".into(), true),
                IndexEntry::new("docs/manual.md".into(), "/r/docs/manual.md".into(), false),
                IndexEntry::new("lib/FooBar.ts".into(), "/r/lib/FooBar.ts".into(), false),
            ],
        }
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
        let index = SearchIndex {
            entries: vec![IndexEntry::new(
                "cafe\u{301}.txt".into(),
                "/r/cafe.txt".into(),
                false,
            )],
        };
        let hits = search(&index, "txt");
        let hit = &hits[0];
        // Chars: c(0) a(1) f(2) e(3) combining(4) .(5) t(6) x(7) t(8).
        assert_eq!(hit.indices, [6, 7, 8]);

        // Fully non-ASCII graphemes go through the grapheme mapping.
        let index = SearchIndex {
            entries: vec![IndexEntry::new("день.txt".into(), "/r/d.txt".into(), false)],
        };
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
