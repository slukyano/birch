---
type: Task
title: Add a scrollbar
status: Draft
priority: medium
description: A scroll indicator down the pane's edge, hidden when everything fits, with a way to turn it off.
---

Maintainer request: birch should show a scrollbar.

## Shape

- **Hidden when everything fits** — a full-height bar carries no information and steals a column.
- **A way to turn it off**: a flag, and/or a config key, and/or a theme axis. Design picks; the
  cheapest coherent option is a setting alongside the other toggles, since it is a preference rather
  than a per-invocation choice.
- **Where it goes**: the right edge is conventional, but the right two columns are the git badge
  gutter today (`render::BADGE_WIDTH`), so the layout has to give it a column or share one.
- **What it is made of**: a block-glyph track (`│`/`█`, or the eighth-block ramp for sub-row
  precision), themed like the guides.

## Notes

- `hit_test` geometry must account for whatever column it takes, exactly as `064`'s left-side badge
  placement would.
- Dragging the bar is a separate, larger question (it needs mouse-drag tracking, which birch has
  never had); the first version can be a pure indicator.
- Interacts with `064` (badge placement) — both compete for the right-hand columns, so whichever
  lands second inherits the layout the first one leaves.
