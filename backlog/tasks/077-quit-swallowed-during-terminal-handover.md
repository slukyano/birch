---
type: Task
title: A quit arriving during a terminal handover is swallowed
description: The stale-event drain that runs after a child returns the tty discards the quit flag, so a SIGHUP or ctl quit delivered while the child was running is answered and then ignored.
status: Draft
priority: medium
---

Found by the independent review of sprint 017, in code the sprint did not touch — it predates
[ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md) and sits on `main`.

## What is wrong

When a file is opened in the terminal, the child owns the tty; on return, `perform_open` drains the
events that queued meanwhile, applying everything except stale input:

```rust
while let Ok(pending) = events.try_recv() {
    match pending {
        AppEvent::Input(_) => {}
        other => {
            self.handle(terminal, events, other);
        }
    }
}
```

`App::handle` returns `true` to mean *quit*, and here the return value is discarded. So an
`AppEvent::Shutdown` (`SIGHUP`/`SIGTERM`, which the design doc requires to quit through the normal
path with state saved and the terminal restored) or a `ctl quit` that arrives while the editor is
open is consumed, answered `ok` to its client, and then ignored: birch keeps running.

`ctl quit` makes it user-visible — the client is told the instance is exiting and it is not.

## Why it is filed rather than fixed

Sprint 017 rewrote the loop around batches, and ADR 0024 decision 4 is precisely about not running
events behind a terminal hand-off — so this is the same subject. It is nevertheless **pre-existing
and outside the sprint diff**, and the fix is not a one-liner: the quit has to travel out of
`perform_open` (which returns nothing today) and through the `NavEffect` path to the loop, without
losing the save-and-restore ordering that a normal quit performs.

## Direction

- Propagate the flag rather than dropping it — `perform_open` reports that a quit was seen, and the
  loop ends through the usual path (`save_persisted`, socket cleanup, terminal restore).
- Prefer folding this drain into the batch model instead of keeping a second, subtly different
  drain: two places that consume queued events by different rules is how this diverged.
- A regression test can drive it without a real child: queue a `Shutdown` before the drain runs and
  assert the loop ends.

## Design

`perform_open` returns nothing, so the quit `handle` reports inside its stale-event drain has
nowhere to go. Three changes:

1. **`perform_open` returns whether a quit was seen.** Its two call sites propagate it: the
   `NavEffect::Open` arm of `handle_action` folds it into the value it already returns
   (`self.picked.is_some()`), and `handle_ctl`'s `CtlEffect::Open` returns it instead of `false`.
2. **The drain becomes the batch drain.** The loop in `perform_open` is a second, subtly different
   consumer of queued events — which is precisely how the two diverged. It is re-expressed with
   `drain_batch`, the helper [ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md)
   introduced, with a closure that discards stale `AppEvent::Input` and reports a quit as
   `BatchStep::Quit`. Nothing may run behind a quit, which is the same rule the batch already obeys.
3. **The quit still exits through the normal path.** Returning the flag reaches `run`'s loop, which
   already performs `save_persisted`, socket cleanup, and terminal restore. No separate teardown.

### Public-surface delta

**None.** `birch ctl quit` already replies `ok`; it now also does what it says.

### Tests

`drain_batch`'s own behaviour is covered from sprint 017. What is new is the closure's mapping and
the propagation: a queued `AppEvent::Shutdown` during a handover yields quit; a queued stale
`AppEvent::Input` is discarded and does not; and a quit stops the drain rather than letting queued
events run behind it.
