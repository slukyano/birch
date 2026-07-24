---
type: Task
title: Split compact chains on demand
description: → on an already-expanded chain un-collapses the middle folders into their own rows; collapsing re-fuses.
status: Done
priority: high
---

Maintainer feature request (initially read as a bug report): with a compacted chain
`a/b/c` already expanded, pressing `→` should un-collapse the middle folders so they
become reachable as individual rows.

## Design

Per [ADR 0014](../docs/adr/0014-chains-split-on-demand.md):

- `→` on an expanded chain row **splits** it: every member dir becomes its own nested
  row, all expanded (middles have exactly one visible child, so expansion is free —
  they are already loaded, or the chain could not have formed). Selection stays on the
  tail row; nothing jumps.
- The split is **render-layer state**: a set of real paths in `FlatView` whose nodes
  refuse to start or join a chain. `visible_rows` receives it through `Decor` — the
  real tree keeps speaking real paths, per the load-bearing boundary.
- **Collapsing re-fuses**: whenever a dir is collapsed (via `←`, Enter-toggle, or
  mouse), the split set drops that path and everything under it, so the chain
  compacts again from that point. Collapsing a middle member re-fuses only the
  sub-chain below it.
- Splits are session-local (not persisted): the persisted expansion of middle dirs is
  invisible inside a fused chain, so a restart shows the familiar compact row.
- Stale split entries (dir deleted while split) never match a row and are dropped the
  next time an ancestor collapses; harmless by construction.

Design doc updates: keyboard table gains `→` on an expanded chain; the compact-folders
bullet "keyboard treats a chain as one node" is amended to name the split gesture and
re-fuse rule.
