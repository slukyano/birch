---
type: Task
title: Adopt the cmux Dock integration
description: Rewrite birch-cmux around cmux's right-sidebar Dock — per-window socket, preview-as-tab, a follow watcher that re-roots on workspace switch — replacing the workspace-split adapter.
status: Done
priority: high
---

The workspace-split cmux adapter (spawn birch in a split, toggle, dedicated preview
pane) had irreducible open flicker — cmux splits are born at 50% width and resized, and
there is no primitive to create a pre-sized split or a split-with-command. cmux's
right-sidebar **Dock** (beta) hosts persistent controls declaratively via `dock.json`
and is the natural home for a file tree: no split, no toggle machinery, its own width.
Maintainer decision (after live design): move cmux integration onto the Dock; no
non-dock fallback (tmux/herdr keep the split-pane pattern).

## Design

Recorded as [ADR 0016](../docs/adr/0016-cmux-integrates-via-the-dock.md). Established
live (throwaway cmux windows) before implementing:

- **One dock birch per window.** The dock's control shell has no `CMUX_WORKSPACE_ID`;
  identity comes from `cmux current-window` (the focused window — which is this window
  whenever you interact with its dock). Socket keyed on `cksum(window-uuid)`, so the
  launch side and any pane in the window compute the same path — no manual
  `BIRCH_SOCKET` export.
- **Three verbs.** `dock-run [root]` is the `dock.json` entrypoint: resolve the window,
  start the follow watcher, `exec` birch (`$BIRCH` names the binary; root defaults to
  the window's selected workspace, since the global `dock.json` can't name a folder).
  `preview <file>` is birch's `--open-cmd`: open the file as a **new tab in the window's
  main pane** — a context-less dock birch's `cmux open` otherwise lands in the dock, so
  the file is routed to an explicit `--pane`. No split (that was the flicker source and
  a bootstrap-terminal bug), no single-preview replacement. `dock-socket` prints the
  window's socket for reverse-reveal (`birch-ctl --socket "$(birch-cmux dock-socket)"
  reveal <file>`).
- **Follow watcher.** A background sibling of the dock birch (an internal shell
  function, not a verb — `dock-run` is the only caller). It listens for
  `workspace.selected` and re-syncs the window's root to its currently-selected
  workspace's directory from authoritative state (the event is only a trigger;
  `window.focused` carries a stale workspace id, so it is not used). It is a singleton
  per dock socket by construction and needs no coordination: it dies with the window
  (shared-pty SIGHUP) and with cmux (the event stream EOFs, no `--reconnect`).
- **No birch changes.** Everything uses existing capabilities: `--socket`,
  `--open-cmd`, `--open-detached`, and `birch-ctl set-root`/`reveal`. The adapter is
  contrib + docs only.
- **Beta dependency.** Requires `rightSidebar.beta.dock.enabled`; the adapter rides the
  Dock beta's observed behavior. Noted in the ADR as the stability risk.
