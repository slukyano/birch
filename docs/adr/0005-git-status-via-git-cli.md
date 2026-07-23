---
type: ADR
title: Git state is a side-table snapshot fed by the git CLI, not gix
status: Accepted
sprint: sprint-002
---

# Context

The design doc's stack note lists gix for git status, and its architecture section declares
everything outside the two load-bearing boundaries negotiable. Evaluating gix for the 0.2
status feature: index-worktree status is available, but HEAD-vs-index status (added/staged
files) needs hand-rolled tree diffing, rename detection is optional machinery, and the API
surface is large and still evolving — a lot of integration work for exactly the semantics
`git status --porcelain=v2` emits natively. The porcelain v2 format is a documented,
stable, machine-readable interface; parsing it is ~100 lines. A `git` subprocess dependency
is acceptable for status display: birch shows git state only where a repo exists, and
machines with repos have git (the same trade VS Code makes). The no-subprocess argument in
the design doc is specific to content search, where the ripgrep crates also supply the
ignore logic.

# Decision

Two parts:

1. **Git state is a side-table, not node fields.** `birch-core` exposes `GitState`: a
   snapshot mapping real paths to file statuses, ancestor-dir rollups (computed from path
   components, so propagation works for collapsed and never-loaded dirs), the
   per-directory lists of deleted-but-tracked files, and the ignored set. The tree stores
   nothing about git; the view-model reads the snapshot at flatten time and injects
   deleted files as synthetic rows.
2. **The snapshot is produced by a worker thread running
   `git status --porcelain=v2 -z --ignored=traditional --untracked-files=all`** and
   parsing the output. No repo → git features off. gix remains the intended replacement
   if the subprocess ever becomes a real constraint — the `GitState` interface is the
   swap point, and a future ADR would supersede this one.

# Consequences

- Status semantics match `git status` exactly (including renames and conflicts) with no
  diffing code to maintain.
- birch degrades gracefully without git on PATH: tree works, badges absent.
- `--ignored=traditional` collapses ignored dirs to one entry; files inside an ignored dir
  inherit dimming from the ancestor in the view instead of being listed individually
  (bounded output even with huge ignored trees).
- A `git` process runs per refresh (debounced); acceptable for a status display that
  refreshes on filesystem quiet periods, not per keystroke.
