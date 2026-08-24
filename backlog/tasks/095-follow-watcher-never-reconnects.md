---
type: Task
title: The dock's follow watcher stops following, silently and for good
description: The watcher subscribes to cmux events once, without --reconnect; the first stream end retires it, and every diagnostic is redirected to /dev/null so nothing says so.
status: Draft
priority: high
---

Reported from live use: the dock birch stops re-rooting to the selected workspace after a while —
"probably after the laptop sleeps". birch itself keeps working; only follow dies. Restarting the
dock control fixes it until the next time.

## The defect, independent of what triggers it

`contrib/birch-cmux:98` subscribes **once**:

```sh
cmux events --name workspace.selected --no-heartbeat 2>/dev/null | while IFS= read -r _; do
```

When that stream ends the `while` loop reads EOF, `follow_watcher` returns, and the backgrounded
subshell exits. Nothing restarts it: `dock-run` has already `exec`ed into birch (`:121`), so there
is no shell left to notice. The dock birch survives — same pty, same socket, same tree — pointed at
whatever root it had when the stream died. That is the reported symptom exactly.

The stream ending is not an accident case; it is the **documented** one. cmux's `docs/events.md`
states the client loop as "reconnect with the latest persisted `seq` if the socket closes", ships
`--reconnect` and `--cursor-file` for it, and lists at least one server-initiated close
(`slow_consumer`, when a subscriber falls behind its 1,024-event pending queue). The adapter uses
neither flag, and the comment at `:92-94` asserts the opposite: "No `--reconnect`, so the stream —
and this watcher — end when cmux quits." Only the *sufficiency* holds. It is not the only way the
stream ends, and the code is written as though it were.

The second half of the defect is that this is **unfalsifiable from the outside**. Every command in
the watcher redirects both streams away (`:73`, `:87`, `:98`, `:103`), so a retired watcher, a
looping-but-inert watcher, and a healthy watcher with nothing to do are indistinguishable without
`ps`. The bug report can only ever be "it broke again". Same silence as
[`085`](085-report-open-cmd-failures.md), one layer out.

## What the trigger is not

Checked against `~/.cmuxterm/events.jsonl` and `.jsonl.1` on the maintainer's machine (≈61k events,
2026-08-22 → 2026-08-23, spanning two hibernate/wake cycles per `pmset -g log`): the event `seq` is
process-local and monotonic, and it **never resets** across either file. So cmux did not restart in
that window — the one stream-ending cause the adapter's comment does account for is not the one
being hit. Whatever ends the stream ends it under a live cmux.

## The other failure family: a watcher that lives and does nothing

A retired watcher is the leading candidate but not the only shape that produces this symptom. These
all leave the loop spinning while every iteration no-ops, and all are equally invisible:

- **A stale `$_win`.** The window UUID is resolved once, at `dock-run` (`:109`), and the watcher
  keys everything on it. If cmux's session restore rebuilds a window under a new UUID,
  `cmux list-workspaces --window "$_win"` yields nothing, `_dir` is empty, and `:102` `continue`s
  forever. A wake that ends in a relaunch is exactly when restore runs.
- **The wrong `$_win` from the start.** `focused_window()` (`:64`) returns the *focused* window, not
  this dock's. When several dock controls start at once — session restore again — they race, and two
  windows can resolve the same UUID. The socket path is keyed on it (`:67-69`), so that is also two
  birches contending for one socket path.
- **A missing `python3`.** `:73` swallows "command not found" and yields an empty `_dir`, forever.
  Ruled *unlikely here*: `/usr/bin/python3` and a Homebrew python3 both stand behind the
  mise-managed one on this machine. Already covered as a fix in
  [`083`](083-birch-cmux-does-three-things.md).

## Fix

**The bar is that follow survives every mechanism this task can name, not that it survives the
likeliest one.** Two are known and both are in scope; a third that only a recurrence can reveal is
handled by making the failure legible rather than by guessing at it.

**1. Reconnect, per cmux's own contract.** `--reconnect` with a `--cursor-file` under `$state_dir`,
so a resumed stream replays what was missed rather than starting blind. The event is only a trigger
and the root is re-read from authoritative state, so a gap costs nothing — but resuming keeps the
first post-reconnect event from being dropped.

**2. Stop trusting a window identity resolved once, at launch.** The stale and wrong-`$_win` shapes
above are defects in their own right, not merely alternative explanations for this report: a watcher
whose window UUID no longer resolves loops forever doing nothing, and two docks that race
`focused_window()` onto one UUID also contend for one socket path. Whatever `$_win` becomes — a
re-resolution on each event, a validity check, or an identity the dock control can state about
itself rather than infer — it must be a thing that can go stale and be noticed, rather than a
constant captured at `:109`. [`086`](086-adr-0016-window-scoped-framing.md)'s live check of whether
`cmux current-window` returns a ref or a UUID decides how much of this is needed: recycled refs make
it a correctness requirement rather than a robustness one.

**3. Make it observable.** Some channel that survives the control's life — a line per re-root, per
reconnect, and per skipped event to a file under `$state_dir`, or the diagnostic stream
[`085`](085-report-open-cmd-failures.md) asks birch for. This is what makes the fix honest: if a
mechanism outside 1 and 2 is the real trigger, the next recurrence names it instead of restarting
the argument. A mechanism found and not fixed becomes its own `Draft` task, named — never a silent
remainder inside this one.

**`--reconnect` makes [`084`](084-follow-watcher-outlives-birch.md) mandatory, not optional.** Today
the stream EOF is the only thing that ever stops a watcher whose birch has gone; reconnecting
removes it and every leaked watcher becomes immortal. The `kill -0 "$parent"` guard from 084 has to
land in the same change, or before it. The two tasks are one seam and should share a sprint.

## Verify first

The mechanism above is read from the code and from cmux's documented contract; the *trigger* is not
observed. Before designing, make the failure legible and catch one:

1. Log reconnects and re-roots to `$state_dir` on the maintainer's own dock, and wait for a
   recurrence. The log distinguishes the two families outright: last line is a re-root and the
   watcher is gone → retired; watcher alive with no lines → inert.
2. On recurrence, before restarting anything: `pgrep -f 'cmux events'`, and compare
   `cmux current-window` against the `$_win` the watcher holds.

Both are cheap and both are answers, not arguments.

## Test

`contrib/tests/cmux-integration.sh` cannot see any of this — it never interrupts the stream. A check
is available once `--reconnect` lands: assert follow works, break the subscription (`pkill -f
'cmux events'` against the throwaway instance), select a workspace, assert follow still works.
That is a real regression test for the fix and needs no sleep to run.

## Related

- [`083`](083-birch-cmux-does-three-things.md) — reshapes this exact watcher; the loop body is
  rewritten there. Sequence the two or do them together.
- [`084`](084-follow-watcher-outlives-birch.md) — the inverse leak, and a prerequisite for the fix.
- [`085`](085-report-open-cmd-failures.md) — the same missing failure channel, one layer in.
- [`086`](086-adr-0016-window-scoped-framing.md) — ADR 0016's window-scoped framing, which the
  stale/wrong-`$_win` shapes above bear on.
