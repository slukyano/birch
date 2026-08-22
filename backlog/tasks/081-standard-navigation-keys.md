---
type: Task
title: Support the standard navigation keys
description: PgUp/PgDn/Home/End do nothing today - bind them, and decide whether a page moves the selection or only the viewport, and what they mean under a live search.
status: Designed
priority: medium
---

`PageUp`, `PageDown`, `Home`, and `End` are dead keys in birch. `map_event`
(`crates/birch-tui/src/input.rs:71`) maps `↑ ↓ → ← Enter Esc Backspace`, `Ctrl-C`, and printables;
everything else falls into `_ => None`. In a tree that can be thousands of rows long, the only way
to the bottom is holding `↓` or spinning the wheel — and the wheel moves the viewport without
moving the selection, so it does not get you anywhere you can act.

These four keys are cheap to bind and unusually safe: they are non-printable, so they do not touch
the reserved-character rule ([ADR 0008](../../docs/adr/0008-q-types-into-search.md)) the way `?`
does for `073-hotkey-reference`. Every list-shaped TUI and every editor file tree has them; their
absence reads as an omission rather than a choice.

## What the design has to settle

**Does a page move the selection, or the viewport?** birch keeps the two decoupled: `↑`/`↓` move
the selection (`view.move_by`), while a wheel scroll takes the count-only fast path and moves the
viewport alone ([ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md)). `PgUp`/`PgDn` are
keyboard navigation, which argues for moving the selection by a page and letting the viewport
follow — the editor-explorer behaviour. Whichever is chosen, `Home`/`End` should match it.

**How big is a page?** `render::tree_viewport_height` is the obvious unit. Decide whether it is the
full height or height minus one row of overlap (the usual reading-continuity trick), and whether a
page that runs past the end clamps to the last row rather than stopping short.

**What do they do under a live search?** This is the sharp one. With a live search, `↑`/`↓` cycle
*matches* and non-matching rows are dimmed and inert
([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)). So a page either steps a
page's worth of *matches* (consistent with `↑`/`↓`, but "a page" then means nothing visual), or
moves a page down the rendered rows and lands on the nearest selectable one. `Home`/`End` have the
same fork: first/last row, or first/last match.

**Scope of `Home`/`End`.** Whether `Home` lands on the root row or the first child under it, and
whether `Ctrl-Home`/`Ctrl-End` need to exist at all as separate bindings. `Shift`-modified variants
are out — range selection belongs to `074-add-multi-selection`, which is itself scope-fenced.

**`--pick` mode** inherits all of it: the picker is the same tree and the same key handling, and a
long picker list is exactly where paging is worth the most.

## Surfaces it touches

New `InputAction` variants and their arms in `map_event`, the dispatch in
`crates/birch/src/app.rs` (`handle_input`), and the binding tables that document keys — `--help`,
the README, and `docs/design.md`. `073-hotkey-reference` will list whatever this task binds, so the
two should agree on names.

## Design

`PageUp`, `PageDown`, `Home`, and `End` become bound keys under one rule that covers both modes:
**they move the selection, and the viewport follows.**

### The selection moves; the wheel stays the only viewport-only motion

`↑`/`↓` move the selection and let `reconcile` scroll it into view; the wheel takes the count-only
fast path and moves the viewport alone ([ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md)).
Paging is keyboard navigation, so it joins the first group — otherwise `PgDn` would leave the
selection behind and the next `↓` would snap the viewport back, which is the wheel's existing
behaviour and reads as a bug when a key does it. `Home`/`End` follow the same rule: they select the
first/last row rather than scrolling to it.

### The unit of movement is whatever `↑`/`↓` move by

This is the whole of the search question, stated once. `↑`/`↓` step **live rows** in a free tree and
**matches** under a query — because under a narrowing nothing else is selectable
([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md), point 4). The page keys
inherit that unit rather than defining a second one:

| | free tree | live search |
|---|---|---|
| `PgDn` / `PgUp` | ± one page of **live rows**; dim rows are stepped over, not landed on | ± one page of **matches**, in tree order |
| `End` / `Home` | the last / first **live row** | the last / first **match** |

Under a query "a page" therefore measures matches, not screen distance. The alternative — move a
page down the rendered rows and land on the nearest match — was rejected: it is non-monotonic (the
same key can move one match or thirty, depending on which ancestors happen to be expanded) and it
cannot reach a match inside a collapsed directory, which is exactly what match stepping exists to
do. A predictable jump that does not correspond to a screen beats a screen-sized jump that does not
correspond to anything.

**Paging clamps; single steps still wrap.** `↑`/`↓` wrap at the ends of the match list, because a
single step off the last match has an obvious destination. A page-sized jump does not: wrapping
would silently teleport across the whole result set, so `PgDn` past the last match stops on it, and
`End`/`Home` are just a page-step large enough to reach the edge. In the free tree `move_by`
already stops at the first/last live row, so clamping is the existing behaviour there.

### Page size

`render::tree_viewport_height(area)` **minus one row of overlap**, floored at 1: the row that was
at the bottom edge becomes the row at the top, which is the reading-continuity convention every
editor uses and the only reason a page is not simply the viewport. The floor matters — a
one-row-high pane must still move.

### `Home` lands on the root row

No special case is needed: the root is a real, selectable row and the first live row, so "the first
live row" delivers it. Landing on the first child instead would be an exception with nothing behind
it. Under a query the root is usually dim, and `Home` then reaches the first *match*, by the same
rule.

### No `Ctrl-Home` / `Ctrl-End`, and `Shift` stays unbound

birch has one tree and one axis, so a second pair of edge keys would have nothing left to mean.
`Ctrl` and `Alt` modifiers on these four keycodes map to the unmodified action — `Ctrl-End` means
the end everywhere else, and silently doing nothing is worse than doing the obvious thing.
**`Shift`-modified variants stay unmapped**, reserved for `074-add-multi-selection`'s range
extension; a `Shift-PgDn` bound now would have to be unpicked there.

### Implementation

- **`crates/birch-tui/src/input.rs`** — four new `InputAction` variants (`PageUp`, `PageDown`,
  `Home`, `End`) and their arms in `map_event`, matching `KeyCode::PageUp`/`PageDown`/`Home`/`End`
  when `SHIFT` is absent.
- **`crates/birch-tui/src/flat_view.rs`** — `FlatView::move_to_edge(&mut self, rows, first: bool)`,
  selecting the first or last live row and asking the viewport to follow. Paging reuses `move_by`,
  which already counts live rows and clamps.
- **`crates/birch/src/app.rs`** — `cycle_match(forward)` generalises to
  `step_match(delta: isize, wrap: bool)`; `↑`/`↓` become `step_match(±1, wrap = true)` and the page
  keys `step_match(±page, wrap = false)`, with the edges expressed as a step of the match count.
  This needs `anchor_index` to stop wrapping its result: it currently folds "past the last match"
  to `0`, which is right for a ±1 wrap and wrong for a clamp. It returns the raw tree-order
  position (`0..=len`) and `step_match` applies the wrap. Behaviour for `↑`/`↓` is unchanged, and
  a test pins that.
- **Dispatch** in `handle_input` reads `render::tree_viewport_height(area(terminal))` the way the
  scroll fast path does. The page keys are *not* added to the `expansion_dirty` marking list: they
  expand nothing in a free tree, and under a query the expansion happens inside `step_reveal`,
  which already marks it.
- **`--pick` inherits everything** with no branch — the picker is the same tree and the same
  dispatch. A long picker list is where paging pays best.

### Tests

`map_event` coverage for the four keycodes and for the `Shift` variants staying `None`; `move_by`
and `move_to_edge` over a fixture with dim rows (page steps over them, `End` lands on the last
*live* row); `step_match` clamping at both ends and the `±1`-wraps-unchanged pin; a page in a
viewport of one row.

### Public-surface delta

No CLI flag, config key, socket field, environment variable, on-disk path, or library API changes.
The user-facing contract that does change is the **key bindings**, in the two places that document
them:

- **`docs/design.md`** — the Keyboard table gains `PgUp` / `PgDn` and `Home` / `End`, each stating
  the search behaviour in the same parenthetical style as the `↑`/`↓` row.
- **`README.md`** — the Quick Start navigation sentence names them.

`--help` carries no key table (the task assumed it did), so nothing changes there.
`073-hotkey-reference` will list these under the names used here: `PgUp`, `PgDn`, `Home`, `End`.
