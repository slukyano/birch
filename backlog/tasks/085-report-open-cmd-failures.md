---
type: Task
title: An open command that fails says nothing
description: birch reports only spawn failure for --open-cmd; a non-zero exit is discarded, so a host adapter has no failure channel at all.
status: Designed
priority: medium
---

`crates/birch/src/app.rs:1448-1465` reports only that the *spawn* failed. The reaper thread does
`let _ = child.wait()` and stderr goes to `/dev/null`, so a command that starts and then fails is
indistinguishable from one that worked.

For the host adapters that is the whole story: `birch-cmux`'s preview path, `birch-tmux open-in-main`
and `birch-herdr open-in-main` all fail silently. "cmux is not running", "no focused window", and
"the open failed" all look exactly like success — you press Enter and nothing happens, with nowhere
to look.

The adapters' own silence is correct locally: they run detached against a live TUI, so writing to
the terminal would corrupt the screen. That makes birch the only place the failure can surface.

## Do

Report a non-zero exit into the status line, through the existing event channel — roughly
"open failed: birch-cmux exited 1". About ten lines.

This does not extend birch's promise to adapters (`docs/design.md` — the socket protocol,
`--socket`, `--open-cmd` with `--open-detached`, clean SIGHUP exit). Adapters call nothing new; birch
stops discarding something it already has. It is birch's own reporting becoming honest, not a new
integration surface.

Open for design: whether stderr is worth capturing for the message, or the exit status alone is
enough; and how the message interacts with the status line's other users.

## Design

### The premise needs one correction

Terminal-mode opens already report a non-zero exit. `open_in_terminal` (`crates/birch/src/app.rs`,
the `Ok(code) if !code.success()` arm) sets `open: {program} exited with {code}`. The gap is
entirely in `open_detached`, which reaps with `let _ = child.wait()` and sends the child's stderr to
`Stdio::null()`.

That is not a narrowing of the task, because **`--open-detached` is the mode every host adapter
uses**. `birch-cmux`, `birch-tmux` and `birch-herdr` all launch birch with it, so the path that
loses failures is exactly the path the adapters run on, and the path that reports them is the one
they never take.

### Where the message comes from

The reaper is a background thread and `status` belongs to `App`, so the failure has to arrive as an
event. birch already has one shape for that: each worker owns a typed channel, and `forward()`
(`crates/birch/src/main.rs`) wraps it into the unified `AppEvent`. Opens get the same treatment —
an `OpenEvent`, an `AppEvent::Open` arm that sets `status`, and a `Sender<OpenEvent>` on `AppWiring`
and `App` that `open_detached` clones into the thread it already spawns.

Two alternatives, both rejected:

- **Hand `App` a `Sender<AppEvent>`.** Fewer types, and it gives the app a way to post any event to
  itself — a general capability bought for one caller, and the one thing the worker-channel pattern
  exists to avoid.
- **Poll with `try_wait()` from the event loop.** No new channel at all, and the status update would
  happen on the thread that owns it. It fails on the loop's own shape: the loop blocks on `recv`, so
  a child that failed while nothing else was happening would go unreported until the next unrelated
  event. A failure that appears when the cursor next moves is worse than no failure at all.

### The message says why, not just that it failed

`open_detached` gets `Stdio::piped()` for stderr, and the reaper reads it before reporting.
`open failed: birch-cmux exited 1` tells nobody anything; `open failed: birch-cmux: no focused cmux
window` is the reason, and it is the sentence the adapter already writes and birch currently
discards. Reporting the exit code alone would satisfy the task's title and not its purpose — the
task exists because *host adapters have no failure channel*, and a channel that cannot carry the
reason is not one.

The text is a child process's bytes arriving in a TUI, so it is bounded and cleaned at the boundary:
first line only, capped, control characters stripped. Same treatment
[`096`](096-open-with-names-the-adapter.md) needs for the display name, for the same reason.

Message shape, matching the wording terminal mode already uses:

| Condition | Status line |
|---|---|
| non-zero exit, stderr has content | `open failed: {program}: {first line of stderr}` |
| non-zero exit, stderr empty | `open failed: {program} exited with {code}` |
| spawn failed | `open failed: {program}: {io error}` (unchanged) |
| exit zero | nothing |

Note that stdout stays `Stdio::null()`. A detached child's stdout is not a failure channel and
capturing it would mean reading a stream that a long-lived child never closes.

### Sharing the status line

Last writer wins, like every other writer, and the message is transient: `handle_input` clears
`status` on every mapped action, so a failure survives until the next keypress. This is already the
right ordering for the common case — the keypress that triggered the open clears the line, and the
failure lands after it.

### Public surface delta

**None.** No CLI flag, config key, socket message or field, environment variable, on-disk path, or
public API. `--open-cmd` and `--open-detached` are unchanged, adapters call nothing new, and no
adapter has to opt in. What changes is that birch stops discarding a result it already has.

### Tests

- The event arm: an `AppEvent::Open` failure sets the expected status text. Pure, no process.
- A detached open of `/bin/sh -c 'echo boom >&2; exit 3'` produces an event carrying the program,
  the code, and `boom`; the status line shows the stderr form.
- A detached open of `/bin/sh -c 'exit 3'` produces the exit-code form.
- A detached open that exits zero produces no event and leaves the status line alone.
- Bounding: a child that writes a long line, several lines, and control characters to stderr yields
  a single capped clean line.

### Settled

**A failure is reported whenever the child exits, with no grace window** — maintainer decision,
sprint-022 design phase. The reasoning that led there:

**Whether a late failure is still worth reporting.** A detached child usually exits in
milliseconds — every adapter does — but nothing guarantees it. A GUI editor launched detached and
closed with a non-zero status an hour later would put `open failed` on the status line with no
visible cause. The alternatives are to report whenever it exits, or to report only within a grace
window after launch.

The grace window is a magic number that would have to be guessed, and the case it protects against
is both rare and self-limiting — the message is cleared by the next keypress.
