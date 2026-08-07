---
type: ADR
title: The search index is a tree, published while it is built
status: Proposed
sprint: sprint-018
---

# Context

`search::build_index` walks the whole root synchronously and returns only when the walk completes,
producing a flat `Vec<IndexEntry>` in which every entry owns its relative path, its absolute path,
and its basename. Measured against a home directory of 825,782 entries:

| | measured |
|---|---|
| serial build, cold cache | 19.1 s |
| serial build, warm cache | 9.4 s |
| parallel build (`build_parallel`, entries constructed) | 5.7 s |
| index size | 263 MB logical, 316 MB peak RSS |

Nothing is searchable until the walk ends, so from a home-sized root search never appears to work
at all — and the status line reports `(no matches)` throughout, which is a claim about the corpus
that cannot be made while no corpus exists.

Neither cost is inherent:

- **Time** has no constant-factor rescue. Parallelising the walk is only 1.66× once entries are
  actually constructed (allocation dominates traversal), which leaves seconds either way.
- **Memory** is roughly ten times what the data requires. The basenames total 16 MB; 825,782
  entries live under 87,231 directories, so a flat list of full paths re-stores every ancestor. A
  parent index plus a slice of a basename slab measures ~25 MB for the same corpus.

fzf is the reference point for how this is supposed to feel, and it is instructive that it is not
frugal — it holds full lines in one slab, which for this corpus would be ~92 MB. It has no faster
walk either. It streams: matching begins while the producer is still walking.

# Decision

**The index is tree-shaped, and it is published while it is being built.**

1. **An entry is a parent and a name.** Each entry carries an index to its parent entry and a slice
   of a shared basename slab, rather than owning full paths. Relative and absolute paths are
   *derived* by walking parents when needed.
2. **Name matching reads the slab; path matching reconstructs.** The default corpus is the simple
   name ([ADR 0013](0013-match-what-is-displayed.md)), which the slab serves directly with no
   allocation. A path-mode query (a `/` in the query) reconstructs each candidate into a single
   reused buffer — never a materialised set of 800k paths.
3. **The index is a list of immutable chunks.** The walker emits chunks as it goes; a published
   chunk is never rewritten, so growth costs no copying and any snapshot stays valid. Parent
   indices are global, so a chunk may reference entries in earlier chunks.
4. **Search runs against whatever has been published.** Results appear while the walk continues,
   and improve as it proceeds.
5. **The index has three states — absent, building, complete — and they are distinguishable.** A
   match count is shown only when the index is complete. `(no matches)` is reserved for a complete
   index that holds none; while building, the interface says so.
6. **An arriving chunk never moves a settled selection.** It may add matches the user can step to,
   but the cursor does not move under a reader's eyes.
7. **The walk stays serial.** 1.66× does not pay for the complexity once results stream.

# Consequences

- **Search from a home-sized root becomes usable**: first results in well under a second rather than
  after 9–19 s, and ~25 MB at rest rather than 263 MB.
- **"No matches" becomes trustworthy**, which is the half of the bug that is a correctness problem
  rather than a performance one.
- **Paths are computed, not stored.** Anything that wants a path pays O(depth) for it. Display and
  path-mode matching are the only callers, and both are bounded by the viewport or by one reused
  buffer.
- **The index gains a lifecycle.** Consumers can no longer assume "index present means index
  complete"; the picker in particular must not silently confirm against a partial corpus.
- **Rebuilds restart it.** A dirty batch discards the chunk list and returns the state to building;
  `REBUILD_THROTTLE` continues to absorb bursts.
- **The tree source and the index remain separate.** The index still does its own walk, because the
  tree is lazily loaded and holds only what has been expanded. Sharing one structure between them
  is a larger change and is not proposed here.

# Alternatives considered

- **Parallelise the walk.** Measured at 1.66× with entries constructed — 9.4 s to 5.7 s. Rejected
  as the primary fix: it leaves the same symptom and adds thread-merge complexity.
- **A path slab in fzf's shape** (full paths, one allocation, 8 B/entry offsets). Simpler than a
  tree and needs no reconstruction, but measures 92 MB against 25 MB for the same corpus.
- **Cap the index** by entry count or depth. Bounds memory, but makes every "no matches" mean "none
  in the part that was indexed" — the same dishonesty this ADR removes, in a new place. Unnecessary
  once the representation is ten times smaller.
- **Index only what the tree has loaded**, deepening in the background. Cheap and always correct,
  but a freshly opened picker has almost nothing loaded, which is exactly the case that fails today.
