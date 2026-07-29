---
type: Task
title: Mouse-wheel scrolling feels broken
status: Draft
priority: high
description: The wheel runs away, heavy overscrolling appears to freeze the pane, and the per-tick distance may not match what other terminal apps do.
---

Maintainer report, from live use: wheel scrolling "generally feels pretty broken — runs away
sometimes. Overscrolling a lot makes it all freeze." The per-tick distance is also in question:
other applications appear to move three lines per tick.

## What is known

- **Three lines per tick is already the constant** (`input::SCROLL_LINES = 3`), so the reported
  difference is not the nominal step. Candidates: several wheel events being coalesced per frame,
  the terminal sending more ticks than expected, or momentum scrolling on a trackpad delivering
  bursts that birch applies in full.
- **`scroll_by` clamps** to `rows.len() - viewport`, so overscrolling should stop, not freeze.
- **Peek-loading is viewport-driven** (`app::request_peeks`): every frame, each unloaded directory
  inside the viewport gets a `SourceCmd::Expand`. Scrolling fast across a large tree can therefore
  emit a burst of load requests, and each arriving snapshot rebuilds the rows. That is the leading
  suspect for both the freeze and the runaway feel, and it grows with how far the wheel travels.
- One snap-back cause was found and fixed in sprint 016 (an index refresh re-revealed the current
  match, pulling the viewport back); the reports above are what remains after that.

## Direction

Reproduce first, with a real terminal — `vhs` cannot send wheel events, so this needs a live
session or a synthetic event feed into `map_event`/the app loop. Then decide between rate-limiting
the peeks, bounding how much scroll one frame may consume, or both.
