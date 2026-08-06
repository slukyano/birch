---
type: ADR
title: The event loop draws once per batch, not once per event
status: Accepted
sprint: sprint-017
---

# Context

The app loop took exactly one event per iteration and ended it with `finish_iteration`, which
rebuilds every visible row (`rows()`, O(all visible rows)) and then draws (O(viewport)).
`handle_input` called `rows()` a second time for the same event. The input thread fed an unbounded
channel, so a burst of events queued without limit and each queued event paid two full row
rebuilds.

Measurement (`069`, a PTY harness feeding synthetic SGR wheel events and timing how long a
keystroke waits behind the burst) put numbers on it. On a 9 156-row tree, a 300-event flick left
the pane unresponsive for **785 ms**; overscrolling and then flicking 1 000 events left it
unresponsive for **3 483 ms**, while continuing to scroll for that whole period. Cost is linear in
visible rows: 4.57× the rows produced 4.3× the latency.

The defect was not where it was expected. Peek-loading, the standing suspect, accounted for ~18 %
(`--no-compact`); git for none (`--no-git`); and a burst of 1 000 `Down` keypresses — no mouse
event at all — froze identically at **3 516 ms**. The wheel was merely the only input device that
emits hundreds of events per second. Any future input path capable of bursting (a trackpad drag, a
held key, a scripted socket client) would have hit the same wall.

# Decision

**An iteration of the loop handles a batch of events and draws once.**

1. **The loop drains what has already arrived.** After a blocking receive, the loop takes further
   events non-blockingly and handles each one, then runs a single `finish_iteration` for the whole
   batch.
2. **Every event is still handled, individually and in order.** Nothing is dropped, merged, or
   summed. Only the *frame* is deferred, so no handler's semantics change.
3. **A batch is bounded by a time budget, not by a count.** Once the batch has consumed its budget
   (~8 ms), it is drawn and the next iteration begins, so a continuous stream keeps repainting
   instead of starving the screen. A budget degrades with machine speed; a fixed maximum event
   count would be an arbitrary number that means different things on different hardware.
4. **An event that hands off or ends the loop closes the batch immediately.** Quitting, and handing
   the terminal to a child process, cannot have queued events processed behind them.
5. **An event computes the row set only if it needs one.** Scrolling needs nothing but the row
   *count* — `scroll_by` reads `rows.len()` and nothing else — so it is served from the count the
   previous `finish_iteration` recorded, which `reconcile` re-clamps before every draw regardless.
   Events that need rows, clicks above all, still compute them.

# Consequences

- **The reported freeze and runaway are gone.** With both parts, the 300-event flick answers a
  keystroke in **4 ms** and the overscrolled 1 000-event burst in **3 ms**, against 785 ms and
  3 483 ms before. Deliberate scrolling is unchanged and still moves exactly `scroll_lines` rows per
  tick.
- **Intermediate frames are not drawn.** A 300-event flick renders the destination, not the
  journey. This is the point: the frames in between were never seen, only paid for.
- **Every input path inherits the contract**, including ones not yet written. `067`'s press/release
  split and `068`'s scrollbar are both defined against "one batch, one frame".
- **Per-event work is now worth auditing.** The linear-in-rows cost stands; the batch merely stops
  paying it hundreds of times per gesture. A future source of expensive per-event work would
  degrade the same way, and the harness from `069` is how it would be caught.
- **The unbounded channel stays.** Backpressure was not needed once an event became cheap, and it
  is the more dangerous design: a bounded channel would block the input thread against the tty.

# Alternatives considered

- **Rate-limit the peek requests** — the task's original direction. Rejected on measurement:
  disabling peeks entirely recovers ~18 % of the latency, so it treats a symptom.
- **Coalesce scroll events by summing their deltas** into one movement. Rejected: it special-cases
  one action, changes what "an event was handled" means, and is unnecessary once a scroll costs a
  clamp instead of a tree walk. The freeze also reproduces without any scrolling.
- **A bounded input channel with backpressure.** Rejected: the input thread would block reading the
  tty, and discarding input is a worse failure than deferring a frame.
- **A frame clock — draw at a fixed rate, independent of events.** A larger change to the loop's
  shape, and unnecessary here; `066` (animated themes) may need one later, and would build on this
  contract rather than replace it.
- **Cap the batch by event count instead of time.** Rejected as arbitrary, per decision 3.
