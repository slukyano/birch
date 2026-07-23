---
type: ADR
title: The view-model (flattening, selection, visibility) lives in birch-tui
status: Accepted
sprint: sprint-001
---

# Context

The design doc fixes the real-tree/render split: `birch-core` speaks real paths and must
build without ratatui; compaction, dimming, badges, and highlighting are paint-time. It does
not say where the layer between them lives — the "view-model" that flattens the expanded
tree into visible rows, applies visibility settings (hidden/noise), tracks selection and
scroll, and implements navigation semantics.

Two candidates: `birch-core` (pure logic, testable) or `birch-tui` (render side).

# Decision

The view-model is a module in **`birch-tui`** (`flat_view`), written as pure logic with no
ratatui types so it stays unit-testable without a terminal. `birch-core` exposes only the
real tree, sources, and deltas.

Rationale: compact folders (already assigned to `birch-tui` by the design doc) must reshape
rows *and* navigation — keyboard treats a chain as one node. If flattening lived in core,
compaction would either leak into core or split across crates. Visibility settings also
affect tree *shape* at the row level ("visibility-aware" compaction), so rows, visibility,
selection, and compaction form one cohesive unit on the render side of the boundary.

# Consequences

- `birch-core` stays small and path-only; the compiler-enforced no-ratatui rule holds.
- Selection is keyed by real path (not row index), so rows appearing/disappearing above the
  selection cannot move it — the stability requirement falls out of the representation.
- Later sources (content search) reuse the same view-model unchanged; only the node payload
  differs.
