---
type: Task
title: The cmux follow watcher outlives the birch it serves
description: The backgrounded watcher is not signalled when the window closes or birch quits, so it keeps forking a cmux call and a python3 on every workspace switch.
status: Draft
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
