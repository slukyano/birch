---
type: Task
title: Add live filesystem and git updates
description: Watch expanded dirs only; tree and badges update in place; selection stays stable.
status: Done
priority: medium
blocked_by:
- 002-build-core-tree-view
---

Phase 0.2 (partial) of [the design doc](../../docs/design.md): watcher-driven live updates for
filesystem and git state. Watch only expanded dirs (the constraint shapes the architecture);
selection stays stable when rows appear/disappear above it.

Design-phase open question (from the design doc): selection stability details under heavy
churn — build systems generating files while the user navigates.

## Design

Per [ADR 0006](../../docs/adr/0006-snapshot-deltas-stateless-sources.md), a watcher event
degrades to "re-scan one level of one dir": the `Snapshot` delta carries the authoritative
listing and `Tree::apply` reconciles (insert/update/remove) while preserving surviving
children's state.

**birch-core** gains a `watcher` module wrapping `notify`:

- `Watcher::spawn(cmds) → events`: commands `Watch(dir)` / `Unwatch(dir)` (each dir
  watched non-recursively), events are **debounced dirty-dir batches** — raw notify events
  are mapped to their parent dir sets and flushed after an ~80 ms quiet slice, so a `cargo
  build` storm becomes a handful of re-scans.
- The git repo's `.git` dir is watched (non-recursively) when git is on; any event there
  marks git dirty. Git dirtiness also piggybacks on every fs dirty batch (a save both
  changes the dir listing and the diff), flushed on a slower ~250 ms debounce to the
  GitWorker's `Refresh`.

**birch** (app wiring):

- Expand → `Watch(dir)` unless `is_ignored(dir)`; collapse → `Unwatch` of the dir and
  every watched descendant (watch only *expanded* dirs, not merely loaded ones). The root
  is watched from startup.
- Dirty dir batch → for each dir still loaded in the tree: `SourceCmd::Expand(dir)`
  (re-scan); unknown dirs are dropped.
- Selection stability under churn is already structural (path-keyed selection, follow-
  only-on-move scrolling from sprint 001): rows shifting above the selection cannot move
  it, and the viewport only chases the selection when the user moves it. That answers the
  design doc's open question for the MVP; anything beyond (e.g. pinning the viewport to
  content rather than offsets during storms) waits for real-world friction.

**Tests**: debouncer coalescing logic (pure, no filesystem); an end-to-end watcher test
against a temp dir (create/remove file → dirty batch arrives) marked to tolerate watcher
latency; tree reconcile-preserves-state cases live with the `Snapshot` tests.
