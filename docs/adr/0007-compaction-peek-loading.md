---
type: ADR
title: Compact chains form via bounded peek-loading of only-child dirs
status: Accepted
sprint: sprint-002
---

# Context

Compact folders render a single-child dir chain (`a/b/c`) as one row. Deciding whether `a`
compacts requires knowing `a`'s children before the user expands `a` — but the tree loads
lazily, and recursive eager loading is exactly what the architecture forbids on big trees.

# Decision

**Peek-loading**: when a dir's listing arrives and its only visible child is a single
unloaded dir, the app requests that child's load (a plain `Expand` command — loading a
dir's entries does not mark it expanded). The cascade continues only while the
single-visible-child condition holds, so it is bounded by chain length, not tree size.
Ignored dirs are never peek-loaded (they are never auto-expanded and their interiors are
lazy until explicitly opened). Compaction itself stays a pure flatten-time transform in the
view-model: a chain renders as one row labeled with joined names, keyed by the tail's real
path, expanded iff the tail is expanded; visibility settings participate in the
"only *visible* child" test, and chains split/fuse automatically because flattening
recomputes from the live tree every frame.

## Amendment (sprint-002 implementation)

Peeking only single visible children turned out to be insufficient: a *collapsed* dir row
directly under an expanded parent (the `src/main/java/…` case) must also compact, and that
requires its own listing. The rule as implemented: **every unloaded, non-ignored dir row
currently inside the viewport gets a one-level load request**. This stays bounded — by the
viewport height, not the tree — deduplicates via a requested-set, and subsumes the
original single-child cascade (an unloaded chain tail is itself a visible dir row).

# Consequences

- Cost per chain is one extra readdir per link — negligible; deep artificial chains load
  linearly, never fan out.
- The real tree, watcher, git state, and future search/ops keep speaking real paths;
  nothing outside the flatten transform knows chains exist.
- Segment-level mouse behavior (clicking `b` inside `a/b/c`) becomes meaningful with the
  context menu and lands with the 0.5 file-operations task; until then a chain behaves as
  its tail for all interactions.
