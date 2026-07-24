---
type: Task
title: Add compact folders
description: VS Code-style single-child dir chain compaction as a render-layer transform.
status: Done
priority: medium
blocked_by:
- 002-build-core-tree-view
---

Phase 0.2 (partial) of [the design doc](../../docs/design.md): pure single-child dir chains
render as one row (`a/b/c`), visibility-aware, keyboard treats a chain as one node, chains
split/fuse on live updates. Strictly a paint-time transform — the real tree is untouched.

## Design

Per [ADR 0007](../../docs/adr/0007-compaction-peek-loading.md):

- **Flatten-time transform** in `flat_view`: while a visible dir's only visible child is a
  single dir, extend the chain; emit one `Row` labeled with the joined names (separators
  rendered dim), `path` = tail, `expanded` = tail's flag, children under the tail. The
  chain's head depth is the row depth. `Row` records the chain's member paths so later
  tasks (segment clicks, F2 full-fragment rename) have the real paths without re-deriving.
- **Peek-loading** in the app: when a `Snapshot` for dir `D` arrives and `D`'s only
  visible child is one unloaded, non-ignored dir, request its load (plain `Expand`; loading
  ≠ expanding). A `requested_peeks` set prevents duplicate requests; the cascade is
  bounded by chain length. An unloaded tail renders as a chain of the known prefix and
  extends in place when the peek lands.
- **Interaction**: the chain row is its tail — expand/collapse/Enter/click all act on the
  tail (`←` collapses the tail back to one collapsed chain row). Badges/dim use the head's
  rollup (it covers the whole chain's subtree). Segment-targeted mouse behavior explicitly
  arrives with the context menu (0.5).
- `--no-compact` flag; `Settings::compact` (default on). Compaction applies in every
  source; picker/search interplay is those tasks' concern (matching against compacted
  labels is specified for fuzzy search).

**Tests**: chain forming (2- and 3-link), visibility-awareness (hidden child breaks or
reshapes the chain per settings), split/fuse when a second child appears/disappears via
`Snapshot`, unloaded-tail prefix rendering, no compaction across the root, `--no-compact`
off-switch, and keyboard semantics on a chain row.
