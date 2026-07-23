---
type: Task
title: Add git status badges
description: VS Code-style badges and colors with propagation to ancestor dirs; deleted-but-tracked files shown.
status: Done
priority: medium
blocked_by:
- build-core-tree-view
---

Phase 0.2 (partial) of [the design doc](../docs/design.md): git status badges + colors
(modified, added, untracked, deleted, renamed; ignored dimmed), status propagating to
ancestor dirs, deleted-but-tracked files rendered in deleted state.

Also from the defaults table: gitignored files shown dimmed and **auto-collapsed** by
default (`--hide-ignored` to hide), `--no-git` to disable; ignored dirs are never
auto-expanded, searched, or recursively watched.

## Design

Per [ADR 0005](../docs/adr/0005-git-status-via-git-cli.md), git state is a side-table
snapshot produced by a worker thread shelling out to
`git status --porcelain=v2 -z --ignored=traditional --untracked-files=all`.

**birch-core** gains a `git` module:

- Repo discovery: walk up from the root looking for `.git`; none (or no `git` on PATH, or
  `--no-git`) → git features off, no errors.
- `GitState` (immutable snapshot behind `Arc`): `status_of(path) → Option<FileStatus>`
  (`Conflicted | Deleted | Renamed | Modified | Added | Untracked` — that order is also
  badge severity), `dir_status(path)` from rollups bumped along path components at parse
  time (works for collapsed and unloaded dirs), `deleted_in(dir)` listing
  deleted-but-tracked names per parent, and `is_ignored(path)` — true when the path or any
  ancestor is in the ignored set (`--ignored=traditional` collapses ignored dirs, so
  descendants inherit).
- Porcelain v2 parsing: `1` entries map via staged/unstaged XY (any `D` → Deleted, else
  `A` → Added, else Modified), `2` → Renamed, `u` → Conflicted, `?` → Untracked, `!` →
  Ignored. NUL-delimited; rename entries carry two paths.
- `GitWorker::spawn(repo_root, cmds) → events`: on `Refresh`, run + parse + emit
  `Arc<GitState>`; consecutive refreshes are collapsed by draining the command queue.

**birch-tui**: `Row` gains `status`, `ignored`, and `missing` (deleted-but-tracked).
Flatten consults the snapshot: per-file letter badge (M/A/U/D/R/C) right-aligned, name
colored by status; dirs with changed descendants get a colored `●` badge; ignored rows
dimmed; deleted files injected as synthetic rows merged in sort order under their expanded
parent (selection works on them; opening one reports a status message instead).

**birch**: `--no-git`, `--hide-ignored` flags; `Settings { git, show_ignored }`. Opening a
deleted row and expanding ignored dirs stay manual-only; the watcher and peek-loading
(other tasks this sprint) consult `is_ignored`.

**Tests**: porcelain parsing (all entry kinds, NUL handling, rename pairs), rollup
severity and propagation depth, `deleted_in` grouping, ancestor-based `is_ignored`;
view-side: badge/dim/synthetic-row flattening against a fixture snapshot.
