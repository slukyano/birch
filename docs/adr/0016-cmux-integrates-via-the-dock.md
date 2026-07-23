---
type: ADR
title: cmux integrates via the Dock, not a workspace-split adapter
status: Accepted
sprint: sprint-011
---

# Context

The original `birch-cmux` followed the reference adapter pattern (ADR-less, shared with
tmux/herdr): spawn birch in a workspace split, wire a toggle, render previews in a
dedicated split pane. Live use exposed two structural problems. First, **flicker on
open**: cmux splits are always born at 50% width and must be resized afterward, and cmux
exposes no primitive to create a pre-sized split or a split that launches a command — so
a fresh split visibly appears at half width before narrowing. Second, **complexity**:
per-workspace socket keying, focus discipline, and preview-pane bookkeeping for
behavior the host arguably should own.

cmux's right-sidebar **Dock** (a beta feature, `rightSidebar.beta.dock.enabled`) hosts
persistent terminal controls declaratively from `dock.json`. It is the natural home for
a file tree: a dedicated region with its own width, no split, no toggle machinery. An
extended live design session (throwaway cmux windows) established the load-bearing
facts: the Dock is **per-window** (one control instance each); the dock control's shell
has **no `CMUX_WORKSPACE_ID`** (identity must come from `cmux current-window`); a file
opened from the dockless context lands **in the dock** unless routed to an explicit
workspace pane; **`workspace.selected`** events carry the active workspace (while
`window.focused` carries a stale workspace id); and `birch-ctl set-root` re-roots a live
birch.

# Decision

- **cmux integrates via the Dock.** `dock.json` declares birch as a control; cmux owns
  its lifecycle. `birch-cmux` is rewritten to three verbs:
  - `dock-run [root]` — the `dock.json` entrypoint: resolve the window, start the follow
    watcher, then `exec` birch. Root defaults to the window's selected workspace (the
    global `dock.json` can't name a folder).
  - `preview <file>` — birch's `--open-cmd`: open the file as a **new tab in the
    window's main pane** (routed with an explicit `--pane`). No split, no replacement.
  - `dock-socket` — print this window's control socket, for reverse-reveal / scripting.
- **One birch per window**, socket keyed on `cksum(window-uuid)` — computable at launch
  and from any pane in the window, so no manual `BIRCH_SOCKET` export.
- **A per-window follow watcher** (an internal background sibling of the dock birch, not
  a public verb) re-roots the tree on `workspace.selected`, reading the new root from
  authoritative state. It is a singleton per dock socket by construction: it dies with
  the window (shared-pty SIGHUP) and with cmux (the event stream EOFs — no
  `--reconnect`), so there is nothing to coordinate or leak.
- **No non-dock fallback.** The cmux adapter is dock-only. The tmux and herdr reference
  adapters keep the workspace-split pattern.
- **No birch changes.** The integration uses only existing capabilities — `--socket`,
  `--open-cmd`, `--open-detached`, `birch-ctl set-root`/`reveal`. It is contrib + docs.

# Consequences

- **Rides the Dock beta.** The adapter depends on the beta's observed behavior:
  `dock.json` schema, per-window seeding-on-show, `cmux current-window`,
  `workspace.selected`, and the absence of a window id in the dock shell's env. If cmux
  changes these, the adapter needs updating — birch itself does not.
- **Flicker-free and simpler**: no split/resize, previews open in the main area as
  tabs, the tree follows the active workspace, and the adapter shrinks by ~40%.
- **One birch per window, not per workspace**; the watcher keeps its root in sync, so
  switching workspaces re-points the same tree rather than spawning trees.
- birch's adapter promise is unchanged; the reference adapter pattern (ADR-shared with
  tmux/herdr) remains documented as the general case, with cmux now the Dock-based
  exception.
