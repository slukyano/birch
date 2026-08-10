---
type: Task
title: A preview opens in the workspace you are in
description: The cmux adapter routes previews to the window's first terminal pane, which is usually a different workspace than the selected one.
status: Draft
priority: medium
---

Maintainer goal for the cmux integration: birch sits in the sidebar, follows the current
workspace's directory, and **opens previews as a new tab in the current workspace**. The first two
hold; the third does not.

## What happens instead

`birch-cmux preview` routes to what it calls the window's *main pane*, which
`main_pane()` computes as the **first** terminal pane anywhere in the window's tree. A cmux window
holds many workspaces, so that is whichever workspace happens to come first — not the selected one.

Observed against a live window: the selected workspace was the unnamed one rooted at
`~/workspaces/personal/birch`, while `main_pane()` returned `pane:3`, belonging to
`workspace:3 "Group 2"`. A preview opened from the dock birch lands in the wrong workspace.

This is the documented behaviour, not an oversight in the code —
[`docs/integrations.md`](../../docs/integrations.md) says "opens the file as a tab in the window's
main pane", and [ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md) settles the Dock
design around it. Correcting it therefore amends both.

## Direction

Resolve the target the same way the follow watcher already resolves the root: from the **selected
workspace**, which `cmux list-workspaces --window <id> --json` reports directly and which the
adapter already parses for `current_directory`. The pane then comes from that workspace's subtree
rather than from the window's.

Open for design:

- **Which pane inside the selected workspace**, when it holds several — the focused one is the
  obvious candidate, and `cmux tree` marks it `[focused]`.
- **What happens when the selected workspace has no terminal pane at all** (a browser-only
  workspace, say): fall back to the window's first terminal pane as today, open in the workspace
  regardless, or do nothing and say so.
- **Whether one `cmux` invocation can do it.** Today's `preview` shells out three times
  (`current-window`, `tree`, `open`); resolving through `list-workspaces` should not make that
  worse, since the workspace lookup already exists in the follow watcher.
