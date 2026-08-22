---
type: Task
title: An open command that fails says nothing
description: birch reports only spawn failure for --open-cmd; a non-zero exit is discarded, so a host adapter has no failure channel at all.
status: Draft
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
