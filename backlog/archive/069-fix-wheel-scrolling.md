---
type: Task
title: Input bursts freeze the pane
status: Done
priority: high
description: A burst of input freezes the pane and scrolling runs on after it stops; the wheel is where it shows, but the keyboard freezes identically.
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

The task was retitled during design: the defect is not wheel-specific. A burst of 1 000 `Down`
keypresses freezes the pane identically, so the wheel is only where it shows.

## Design

### Reproduction

A synthetic wheel feed drives a release binary over a PTY, writing the SGR sequences a terminal
emits for a wheel tick (`ESC [ < 65 ; col ; row M`) at a controlled rate. Every byte birch writes
back is timestamped. The decisive metric is not how long drawing continues — an overscrolled pane
draws nothing while still consuming events — but **how long a keystroke waits**: the burst is
followed immediately by a printable character, which must open the search prompt and repaint.

Two fixtures: a flat listing of 2 001 rows, and a 9 156-row tree with every directory expanded
(seeded through `XDG_CACHE_HOME`, so no real cache is touched). Terminal 45×100, release build.

| Scenario | Fixture | Keypress latency |
|---|---|---|
| 30 ticks @ 12 ms (deliberate scrolling) | 2 001 rows | none measurable |
| 300 ticks @ 1 ms (a flick) | 9 156 rows | **785 ms** |
| overscrolled, then 1 000 ticks @ 0.4 ms | 2 001 rows | **776 ms** |
| overscrolled, then 1 000 ticks @ 0.4 ms | 9 156 rows | **3 483 ms** |

Both reported symptoms reproduce exactly. Scrolling continues for 0.8–3.5 s after the input stops
(the runaway), and for that whole period no keystroke is answered (the freeze).

### What the measurements rule out

- **Not the step size.** Every tick moves exactly 3 rows, at every rate tested — 30 ticks advance
  the top row by 89. `SCROLL_LINES` is not implicated.
- **Not peek-loading**, the stated leading suspect. `--no-compact` disables `request_peeks`
  outright: 3 483 ms → 2 861 ms. Peeks are ~18 % of the cost, not the cause.
- **Not git.** `--no-git`: 3 439 ms, within noise of the default.
- **Not the wheel.** A burst of 1 000 `Down` keypresses, with no mouse event at all, freezes for
  3 516 ms — indistinguishable from the wheel. The wheel is merely the only input device that
  emits hundreds of events per second.

### Root cause

**One event costs one full pipeline pass, and nothing coalesces.** The loop in `run` takes a single
event per iteration and ends it with `finish_iteration`, which rebuilds every visible row via
`rows()` — O(all visible rows) — and then draws, which is only O(viewport). `handle_input` calls
`rows()` a *second* time for the same event. The input thread feeds an unbounded channel, so a
burst queues without limit and each queued event pays two full row rebuilds.

Cost is therefore linear in visible rows, which the fixtures confirm: 4.57× the rows produces 4.3×
the latency (0.39 µs per row per event). Overscrolling is the worst case precisely because clamped
events change nothing on screen — the work becomes invisible while the queue still drains.

### The fix

Recorded as **[ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md)**: an iteration of
the loop handles a *batch* of events and draws once. Two independent changes; the measured effect
of each is on the 9 156-row fixture.

1. **Coalesce the queue.** After handling an event, drain what has already arrived and draw once
   for the batch. Every event is still handled, in order — only the frame is deferred, so no
   semantics change. Flick 785 ms → **90 ms**; overscroll 3 483 ms → **1 075 ms**.

   The batch is bounded by a **time budget of ~8 ms**, not by an event count: once the budget is
   spent the frame is drawn and the next iteration begins, so a continuous stream keeps repainting
   rather than starving the screen. A budget degrades with machine speed where a fixed count is an
   arbitrary number that means different things on different hardware. Quitting, and handing the
   terminal to a child, close the batch immediately — queued events must not be processed behind
   them.
2. **A scroll must not rebuild rows.** `scroll_by` consumes nothing but `rows.len()`, so the scroll
   path can be served from a row count cached by the previous `finish_iteration`, which re-clamps
   through `reconcile` regardless. This removes the residual per-event rebuild that change 1 leaves
   behind. Combined: flick **4 ms**, overscroll **3 ms** — a keystroke is answered immediately, and
   deliberate scrolling still moves exactly 3 rows per tick.

Events that genuinely need rows — clicks above all — keep computing them, so hit-testing is
unaffected.

### Public-surface delta

**None.** No new or changed CLI flag, config key, socket field, environment variable, or on-disk
path. `SCROLL_LINES` keeps its value and stays a constant here;
[`075`](075-configurable-scroll-speed.md) turns it into a setting afterwards, and carries that
public surface itself.

### Not verified here

How many SGR events one physical trackpad gesture produces in a real terminal is **unmeasured**.
Synthetic `CGEvent` scrolls cannot stand in: momentum phases originate in the trackpad driver, so
the per-gesture event count needs a hand on real hardware. It bounds only the perceived *distance*
of a flick, not the freeze — and since every tick provably moves 3 rows, birch's distance per event
is whatever the terminal reports, exactly as for any other application.
