---
type: Task
title: A preview opens in the workspace you are in
description: The cmux adapter routes previews to the window's first terminal pane, which is usually a different workspace than the selected one.
status: Done
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

## Design

**The fix is a deletion.** `cmux open` already resolves exactly the target this task wants, and the
adapter's hand-rolled pane resolution predates that ability. `preview` becomes:

```sh
cmux open "$2" --window "$win"
```

`main_pane()` and its awk parser go away, and with them one of the three `cmux` invocations per
file open.

### Why that lands in the current workspace

Read from cmux's own source rather than inferred. `cmux open` sends `file.open` with only the
selectors it was given (`CLI/cmux_open.swift`), and the app resolves the rest
(`Sources/FileOpenSocketSupport.swift`, `v2FileOpen`):

1. **Window** — from `window_id`, which we pass.
2. **Workspace** — `workspace_id` → `surface_id` → `pane_id` → **`tabManager.selectedTabId`**.
   We pass none of the first three, so it is the window's *selected* workspace.
3. **Pane** — with no `pane_id` and no `surface_id`:
   `ws.bonsplitController.focusedPaneId ?? allPaneIds.first`. The **focused pane** of that
   workspace, falling back to its first.

That answers both open questions for free: the focused pane is chosen when a workspace holds
several, and a workspace always has at least one pane, so the "no terminal pane" case reduces to
"whatever pane is there" — a file preview is a tab, and does not require a *terminal*.

**Passing `--window` is load-bearing beyond naming the window.** In `cmux_open.swift` both
environment fallbacks are guarded on `parsedArgs.window == nil`:

```swift
let workspaceRaw = parsedArgs.workspace ?? (parsedArgs.window == nil ? env["CMUX_WORKSPACE_ID"] : nil)
let shouldInheritCallerSurface = parsedArgs.workspace == nil && parsedArgs.pane == nil && parsedArgs.window == nil
```

So `--window` is what suppresses inheritance of the *caller's* surface — the dock birch's own — and
that inheritance is precisely the "files land in the dock" behaviour
[ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md) worked around with an explicit
`--pane`. The workaround is no longer needed; `--window` alone does the job the ADR wanted.

### Why not resolve the workspace ourselves

Passing an explicit `--workspace` was the obvious alternative and is **worse**: any id we read is
already stale by the time `cmux open` runs, whereas cmux resolves "selected" on the main actor
inside the same call, atomically. Explicitness would buy nothing and reintroduce a race. The cost
is that the adapter depends on a resolution order cmux documents only in code — recorded in a
comment at the call site, and covered by [`080`](080-verify-cmux-integration-live.md).

### Confirmed on a live cmux, not only in the source

Both halves were observed against a throwaway instance (the harness is
[`080`](080-verify-cmux-integration-live.md)'s design):

- **The bug, end to end.** birch running in the Dock, `AlphaWs` selected, `enter` on a file — the
  preview opened in `workspace:1`, a different workspace, which `cmux open` then *selected*, so the
  user is moved as well as misrouted.
- **The fix.** With `BetaWs` selected, `cmux open FILE --window window:1` issued from a dock-shell
  environment put the preview in `BetaWs`, in its focused pane.

That environment matters, and measuring it settled the last doubt: the dock shell exports
`CMUX_SURFACE_ID` for the dock surface and `CMUX_WORKSPACE_ID` set to the **window** uuid — a dock
routing id that `resolveWorkspace` explicitly rejects. So without `--window`, the call either lands
in the dock or resolves no workspace at all. That is ADR 0016's "files land in the dock", measured.

### What this changes elsewhere

- **[ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md)** — the `preview` bullet says
  "a new tab in the **window's main pane** (routed with an explicit `--pane`)". Both halves are
  now wrong. Amending an `Accepted` ADR is a maintainer decision.
- **[`docs/integrations.md`](../../docs/integrations.md)** (the verb table) and
  **[`README.md`](../../README.md)** (the cmux blurb) repeat "main pane".
- **`CHANGELOG.md`** — a fix entry.

Nothing in `birch` itself changes; this is contrib plus docs, as ADR 0016 intended.
