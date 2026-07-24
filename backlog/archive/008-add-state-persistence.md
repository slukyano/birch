---
type: Task
title: Add state persistence
description: Persist expansion, selection, and scroll per root; restore on launch.
status: Done
priority: low
blocked_by:
- 002-build-core-tree-view
---

Phase 0.4 (partial) of [the design doc](../../docs/design.md): persist per-root state at
`~/.cache/birch/<root-hash>.json`, keyed on real paths so visibility toggles don't corrupt
restored state. Crash/reboot resilience for an always-running pane.

## Design

- **File**: `~/.cache/birch/<fnv1a-64(root)>.json` (`$XDG_CACHE_HOME` respected). FNV-1a
  is hand-rolled (8 lines) because `DefaultHasher` output is not guaranteed stable across
  Rust releases. Serde/serde_json land as workspace deps (the socket's NDJSON protocol
  needs them next sprint anyway).
- **Schema** (`version: 1`): expanded dirs, selection, and scroll — all paths
  root-relative real paths, so compaction/visibility changes can't corrupt restored
  state. Unknown versions or parse failures are discarded silently (state is a cache).
- **Write policy**: crash resilience without per-keystroke writes — save whenever the
  expansion set changes (expand/collapse are rare, the file is tiny, and expansion is the
  expensive-to-rebuild part), plus once on quit for selection/scroll. Atomic
  write-to-temp-then-rename next to the target.
- **Restore**: on launch, load the file; expanded dirs go into a restore set — when a
  `Snapshot` arrives, any new child dir in the set is expanded and its load requested,
  cascading down the restored subtree as listings arrive. Selection/scroll apply
  directly; the existing sync logic tolerates paths that no longer exist. Restored
  expansion of a dir that became ignored since is dropped (never auto-expand ignored).
- Picker mode neither loads nor saves state (see `add-picker-mode`).

**Tests**: round-trip serialization, corrupt/unknown-version discard, restore cascade
against a scripted source, ignored-dir restore dropped, atomic-write behavior.
