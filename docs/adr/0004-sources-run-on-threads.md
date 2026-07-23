---
type: ADR
title: Sources run on worker threads and feed one app-event channel
status: Accepted
sprint: sprint-001
---

# Context

"Sources are delta streams" is a load-bearing boundary, but the design doc leaves the
concurrency model open. The Files source could answer expand requests synchronously in 0.1
(readdir is fast), but content search (0.6) is inherently asynchronous — debounced,
cancellable, streaming — and the watcher (0.2) delivers events from its own thread. An
interface that only works synchronously would need a rewrite exactly when the second source
arrives, which is the failure mode the boundary exists to prevent.

# Decision

Every source runs on its own worker thread from day one. The contract:

- The app sends `SourceCmd` values (`Expand(path)`, `Refresh`, …) over an mpsc channel.
- The source emits `Vec<TreeDelta>` batches into the single unified app-event channel
  (`AppEvent::Deltas`), which also carries terminal input (`AppEvent::Input` from a
  dedicated input-reading thread).
- The main loop is a plain `recv()` loop over `AppEvent` — no async runtime, no select over
  multiple channels, std `mpsc` only.

# Consequences

- The Files source in 0.1 is trivially simple (recv → readdir → send) but already exercises
  the real interface; content search and the watcher plug in without touching the loop.
- No tokio/async-std dependency; birch stays a lean threaded program.
- Deltas arrive asynchronously even for expand, so the view must tolerate "expanded but
  children not yet arrived" — enforced from the start, which is exactly the discipline live
  updates need later.
