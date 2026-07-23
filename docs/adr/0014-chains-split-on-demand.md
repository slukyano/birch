---
type: ADR
title: Compact chains split on demand — → un-collapses, collapse re-fuses
status: Accepted
sprint: sprint-008
---

# Context

ADR 0007's compact chains render `a/b/c` as one row, and the design doc said "keyboard
treats a chain as one node (the tail)". That leaves middle folders unreachable from the
keyboard: you cannot select `b` to scope an action to it or expand the tree at that
depth. First use surfaced this as a maintainer feature request: pressing `→` on an
already-expanded chain should un-collapse the middle folders (mouse segment-clicks were
already planned, but only arrive with the context menu, and keyboard parity is a
product principle).

# Decision

- **`→` on an expanded chain splits it**: every member renders as its own nested row.
  All members are marked expanded in the real tree (each has exactly one visible child,
  and all members were already loaded — otherwise no chain would have formed), so the
  split appears fully open with no I/O. Selection stays on the tail; `→` on a collapsed
  chain still just expands it.
- **The split is render-layer state**: a path set held by the view (`FlatView`),
  consulted at paint time — a node in the set neither starts nor extends a chain. The
  real tree, watcher, git, and persistence remain untouched, per the real-tree/render
  boundary.
- **Collapsing re-fuses**: any collapse of a dir (arrow, Enter-toggle, mouse) removes
  that path and its descendants from the split set. Collapsing the head re-forms the
  full compact row; collapsing a middle member re-fuses only the sub-chain below it.
- **Splits are session-local.** They are not persisted: a fused chain row shows the
  tail's expansion state, so restoring middle-dir expansion without the split set
  changes nothing visibly, and a restart deliberately returns to the compact default.

# Consequences

- Middle chain members become first-class keyboard citizens; future per-member actions
  (context menu, F2 full-fragment rename) get a selection model for free.
- One more piece of view state that visible_rows must receive (via `Decor`); the
  flatten function stays pure.
- A split survives live updates: if the chain would re-form (children unchanged), the
  split set keeps it apart until the user collapses; if the tree changes shape so the
  chain breaks anyway, entries go stale and inert, cleaned up by the next ancestor
  collapse.
