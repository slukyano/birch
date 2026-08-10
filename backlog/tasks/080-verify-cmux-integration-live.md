---
type: Task
title: Verify the cmux integration live
description: Drive a separate cmux instance and confirm the three things the integration promises — sidebar, follow, preview — actually happen.
status: Draft
priority: medium
---

The cmux integration was built across
[`018`](../archive/018-add-cmux-integration.md), [`021`](../archive/021-refine-cmux-adapter.md) and
[`024`](../archive/024-adopt-cmux-dock-integration.md), and each was verified as it landed. What has
never been checked is the whole thing at once, from an *installed* setup, against the three
behaviours the maintainer actually wants:

1. **birch is in the sidebar** — the Dock control starts, and one dock birch exists per window.
2. **It follows the current workspace** — switching workspace re-roots the tree, including after a
   session restore and after a directory rename.
3. **A preview opens in the current workspace** — as a new tab, in the workspace that is selected,
   which is [`079`](079-preview-opens-in-current-workspace.md)'s subject.

Plus the two verbs a person invokes directly: `dock-socket` resolving the same path from the dock
birch and from a pane in that window, and reverse-reveal through it.

## How it must be run

**Never against the maintainer's own cmux instance** (`AGENTS.md`): launch a separate one with
`open -na cmux` and target it by exporting `CMUX_SOCKET_PATH`, plus `CMUX_WORKSPACE_ID` and
`CMUX_SURFACE_ID` from its `cmux tree --id-format both`, for every command aimed at it.

## The part that may not be automatable

The Dock is a beta **UI** surface, and its config seeds on first show rather than into an already
open Dock. Whether it can be revealed from the CLI is unverified. If it cannot, the sidebar step
needs the maintainer at the window — like [`035`](035-verify-herdr-integration.md) — and this task
says so in its result rather than reporting a pass it did not observe.

## Done when

Each of the three behaviours is either observed and recorded, or named as unobservable with the
reason. Anything that fails becomes a fix in this sprint or a `Draft` task with a name.

## Design

**A separate instance turned out not to be launchable.** Three attempts, recorded so nobody repeats
them:

| Attempt | Result |
|---|---|
| `open -na cmux` | No second process. LaunchServices activates the running app. |
| Direct exec of `cmux.app/Contents/MacOS/cmux` | A second process appears, then exits within seconds — the app self-terminates as a duplicate launch. |
| The same with `CMUX_SOCKET_PATH` + `CMUX_ALLOW_SOCKET_OVERRIDE` + `CMUX_DISABLE_SESSION_RESTORE` | Same. The socket override never binds because the process is already gone. |

`CMUX_TAG`, which does give a fully separate socket and state, is gated to DEV builds
(*"refusing to launch untagged cmux DEV"*), and this is the stable app. So `AGENTS.md`'s procedure
cannot be followed on this machine as it stands, and driving the maintainer's own instance is
forbidden. The task splits accordingly.

### What is automatable — a stub `cmux` on `PATH`

The adapter's own contract is fully testable without cmux: put a recording stub named `cmux`
ahead of the real one on `PATH`, replay captured `tree --json` / `list-workspaces --json`
fixtures, and assert the argv the adapter builds.

That pins exactly what [`079`](079-preview-opens-in-current-workspace.md) changes and what a future
edit would silently break:

- `preview` invokes `cmux open <file> --window <win>` — **no `--pane`**, and `--window` present,
  since that flag is what suppresses caller-surface inheritance.
- `dock-socket` and `dock-run` derive the *same* socket path for one window id, and a different one
  for a different window.
- The follow watcher issues `birch ctl … set-root <dir>` for the **selected** workspace's
  `current_directory`, and survives a fixture that returns empty or malformed JSON without exiting.

As a Rust integration test under `crates/birch/tests/`, it runs inside the existing `cargo test`
gate rather than needing new CI wiring. No cmux, no UI, deterministic.

### What is not automatable

The UI half — the Dock rendering birch, previews visibly appearing in the right workspace, the tree
re-rooting on a workspace switch. That needs the maintainer, exactly as [`035`](035-verify-herdr-integration.md)
does for herdr, and this task delivers a **checklist** for it rather than a claimed pass:

1. Point `dock.json` at the installed adapter, open a fresh cmux window, show the Dock — birch
   appears, rooted at the selected workspace.
2. Switch workspaces — the tree re-roots. Switch to one whose directory was renamed — it corrects.
3. Open a file from the dock birch — the preview tab appears **in the workspace you are in**, and
   in its focused pane.
4. From a pane in that window: `birch ctl --socket "$(birch-cmux dock-socket)" reveal <file>` —
   the dock birch reveals it.

If a way to run a second instance is found later, this checklist is what it should run.
