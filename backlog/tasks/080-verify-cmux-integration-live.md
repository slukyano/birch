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
