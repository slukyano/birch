---
type: ADR
title: One search engine — an ignore-walk index matched by nucleo, rendered two ways
status: Accepted
sprint: sprint-003
---

# Context

Fuzzy filename search is whole-tree ("auto-expanding to reveal matches"), so it cannot be
limited to lazily-loaded nodes; it needs its own path index. The design doc requires:
never descend into ignored dirs, matching against compacted labels, and the same engine
powering two render policies — jump (main pane) and filter (picker).

# Decision

- **Index**: a worker thread walks the root with the `ignore` crate (gitignore-aware,
  parallel, never descending into ignored or noise dirs; hidden entries follow the
  hidden-files setting) and produces an immutable `Arc<SearchIndex>` of root-relative
  paths. Watcher dirty batches trigger a throttled full re-walk — incremental patching is
  deliberately deferred until walk cost is a measured problem.
- **Matcher**: `nucleo-matcher` (the Helix engine) scores the query against relative
  paths, smart-case, synchronously in the app loop — filename corpora are small enough
  that per-keystroke matching needs no debounce (that machinery arrives with content
  search, which is a different task).
- **Two render policies over one match list**: the main pane *jumps* — rows stay put,
  matches highlight, non-matches dim, `↑`/`↓` cycles matches and reveals them (expanding
  ancestors); the picker *filters* — matched paths render as a dense flat list. The match
  list, scoring, and reveal logic are shared; only the row assembly differs.
- **Reveal is a primitive**: "expand ancestors, request loads, select the path once it
  appears" lives in the app as `reveal(path)` — the control socket's `reveal` verb (0.4)
  is the same function.

# Consequences

- Search results can point at paths the lazy tree has not loaded; reveal converges over a
  few delta round-trips (pending-reveal state, re-checked per iteration).
- A full re-walk per dirty batch is O(tree) but throttled and off-thread; big-monorepo
  incremental indexing has a clear seam (the index worker) when needed.
- The `ignore` dependency arrives one task early; content search (0.6) reuses it, as the
  design doc planned.
