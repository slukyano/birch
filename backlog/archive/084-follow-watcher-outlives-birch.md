---
type: Task
title: The cmux follow watcher outlives the birch it serves
description: The backgrounded watcher is not signalled when the window closes or birch quits, so it keeps forking a cmux call and a python3 on every workspace switch.
status: Done
priority: medium
---

`contrib/birch-cmux:29-30` and `:93-94` claim the follow watcher "dies with the window (shared pty
SIGHUP) and with cmux (the event stream EOFs)". The second half holds — no `--reconnect` is passed,
so the stream ends when cmux quits. The first half looks wrong.

On hangup the kernel signals the **controlling process**, not every process in the session. After
the `exec` at `:121` that is *birch*. The watcher backgrounded at `:120` is a separate process that
nothing signals: the script `exec`ed away, so no shell survives to signal its jobs, and a
non-interactive `sh` has no job control. It never touches the pty either — every call inside it
redirects output (`:98`, `:73`, `:87`, `:103`) — so no `EIO` or `SIGPIPE` wakes it.

**There is also a leak path that does not need the window to close.** cmux's Dock drops into an
interactive login shell when a control's command exits, so quitting birch with `q` leaves the pty
open, birch gone, and the watcher running for the life of the app.

Cost per leaked watcher: every `workspace.selected` event anywhere forks a `cmux` call plus a
`python3` (`:73`), indefinitely. A few closed windows turn one workspace switch into several
interpreter starts.

## Fix

Do not rely on signal behaviour. `exec` preserves the pid and `$$` is unchanged inside a subshell,
so capture it before backgrounding and check it in the loop:

```sh
parent=$$
follow_watcher "$win" "$sock" &
# first line inside the loop:
kill -0 "$parent" 2>/dev/null || exit 0
```

That makes the comment true by construction. Prefer it to `[ -S "$_sock" ] || exit 0`, which has a
startup race — the watcher starts before birch binds the socket, so the first event would kill it.

## Verify first

This is a mechanism argument plus a negative `killpg` grep of the cmux bundle, not an observation.
Confirm against `contrib/testcmux`: close the window, then check whether `cmux events` is still
running; separately, quit birch with `q` and check the same.

## Test

`contrib/tests/cmux-integration.sh` never closes a window and never quits birch, so it cannot see
this. A check is cheap — close the window, then `pgrep -f 'cmux events'`.

## Design

The mechanism is the one the task specifies — capture `$$` before backgrounding, check `kill -0`
inside the loop — and it lands as part of
[`095`](095-follow-watcher-never-reconnects.md)'s supervision loop rather than as a separate edit,
because the two are the same twelve lines:

```sh
parent=$$
follow_watcher "$win" "$sock" "$parent" &
```

The guard appears twice in that loop, and both are load-bearing:

- **The outer `while kill -0 "$_parent"`** is what makes reconnection safe. Once the watcher retries
  a dropped stream, the stream ending is no longer what stops it, so the liveness check becomes the
  *only* thing that does.
- **The first line of the inner loop** ends a watcher whose birch died while the stream was healthy
  — the `q` case, where the Dock keeps the pty open and drops into an interactive shell, so no
  hangup arrives and nothing else would notice.

Ordering is not optional: this must land with or before the reconnect, never after. A watcher that
retries forever with no liveness check is a worse leak than the one being fixed.

Rejected, as the task already argues: `[ -S "$_sock" ] || exit 0`. It races startup — the watcher
begins before birch binds the socket, so the first event would kill it — and it tests the wrong
thing, since a stale socket file outlives the process that made it.

**Pid reuse** is the one hole and it is left open deliberately. If birch's pid is recycled by an
unrelated process, `kill -0` succeeds and the watcher lives on; it then re-roots a socket that no
longer exists, which fails harmlessly and is logged. Closing it properly needs a handle the shell
does not have, and the cost of the hole is one idle process rather than a wrong tree.

### Public-surface delta

**None.** The change is entirely inside `contrib/birch-cmux`.

### Tests

In `contrib/tests/cmux-integration.sh`, which never closes a window or quits birch today: quit the
dock birch with `q`, settle, then assert `pgrep -f 'cmux events'` finds nothing belonging to the
throwaway. The same check catches a regression in either direction — a watcher that dies too early
fails the follow checks above it.
