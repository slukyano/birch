---
type: Task
title: Add fuzzy filename search
description: Jump-style search on any printable char; dim non-matches, cycle matches, never descend into ignored dirs.
status: Done
priority: medium
blocked_by:
- 002-build-core-tree-view
---

Phase 0.3 of [the design doc](../../docs/design.md): typing starts a whole-tree fuzzy jump —
non-matches dim, matches highlight, arrows cycle, tree stays spatially stable, auto-expands
to reveal matches, matches compacted labels. The same engine later powers picker-mode
filtering.

## Design

Per [ADR 0009](../../docs/adr/0009-search-index-and-engine.md) (engine) and
[ADR 0008](../../docs/adr/0008-q-types-into-search.md) (`q` types; `Ctrl-C` quits).

**birch-core** gains a `search` module:

- `SearchIndex`: immutable list of root-relative paths (with kind), built by an
  `IndexWorker` thread walking the root via the `ignore` crate — gitignore-aware, skips
  noise, honors the hidden setting; never descends into ignored dirs. Watcher dirty
  batches send `Rebuild`; consecutive rebuilds collapse and a rebuild younger than 500 ms
  is skipped.
- `matches(index, query) → Vec<(PathBuf, score)>` on `nucleo-matcher`, smart-case,
  sorted best-first, matching the full relative path (which is also what a compacted
  label shows — matching against `a/b/c` falls out of path matching).

**App / view**:

- Search state: `query`, the sorted match list, current-match pointer, and the
  pre-search (selection, scroll) for `Esc` restore. Any printable char (incl. space)
  appends; `Backspace` deletes; `Esc` clears and restores; `↑`/`↓` cycle matches while a
  query is live; `Enter` keeps its meaning (open/expand the selected row).
- On query change: re-match, jump to the best match via `reveal(path)` — the new app
  primitive that expands ancestors (requesting loads for unloaded ones), sets the
  selection, and re-checks each iteration until the path is visible. The socket's
  `reveal` verb (0.4) reuses it.
- Row assembly gets the match set: matches highlight (bold), everything else dims. The
  status bar shows `search: <query> (n/total)`.
- The tree stays spatially stable: no filtering, no reordering; ignored dirs are never
  expanded by reveal (matches cannot point there — the index never contains them).

**Tests**: index building against a fixture tree (ignored/noise/hidden exclusions),
match scoring order and smart-case, reveal expansion cascade against a scripted source,
search-state transitions (type/backspace/Esc-restore/cycle), and the input-mapping change
(`q` no longer quits; Ctrl-C does).
