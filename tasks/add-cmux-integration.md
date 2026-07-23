---
type: Task
title: Ship the birch-cmux adapter
description: contrib/birch-cmux spawns birch in a cmux split; Enter opens files as cmux preview tabs in the main pane.
status: Done
priority: high
---

The design doc names `birch-cmux` as a planned host adapter but only tmux and herdr
shipped. The maintainer now works inside cmux, making it the live integration testbed.

## Design

cmux (com.cmuxterm.app) is a Ghostty-based terminal with a control socket and a rich
CLI (`cmux`): windows → workspaces → panes → surfaces (tabs). The primitives that
matter here, all verified live:

- `cmux new-split <dir>` splits from the caller's surface and prints the new ref
  (`OK surface:N workspace:M`).
- `cmux respawn-pane --surface <ref> --command <text>` types a command into that
  surface's shell.
- `cmux open <file> --surface <ref>` opens the file as a **preview tab** in the pane
  owning that surface (markdown gets a live-reload viewer, HTML a browser tab).
- `cmux rename-tab --surface <ref> <title>` / `cmux tree --workspace <id>` allow
  find-by-title; `cmux close-surface --surface <id>` kills the pane (birch already
  exits cleanly on SIGHUP).
- Every cmux shell carries `CMUX_WORKSPACE_ID` / `CMUX_SURFACE_ID` (UUIDs).

`contrib/birch-cmux` follows the birch-tmux shape (open / toggle / socket verbs, same
private state dir, same shquote discipline):

- **Spawn**: `new-split left` from the invoking surface, then `respawn-pane` the new
  surface with `birch --socket <sock> --open-cmd 'cmux open --surface <main> {}' <dir>`,
  where `<main>` is the invoker's `$CMUX_SURFACE_ID` — captured at spawn time, so
  open always targets the pane the user launched from. `rename-tab` marks the surface
  `birch-tree` for later discovery; a best-effort `resize-pane` narrows the split.
- **Open target is a preview tab, not `$EDITOR`**: in cmux the main terminal usually
  runs an agent, so typing an editor command into it (the tmux adapter's trick) would
  collide with a running process. `cmux open` renders into a tab without touching any
  terminal. Users who want an editor can still pass any `--open-cmd`.
- **Toggle**: find the `birch-tree` surface in the caller's workspace via `cmux tree`;
  close it if present, spawn otherwise.
- **Socket**: `birch-cmux-<CMUX_WORKSPACE_ID>.sock` in the adapter state dir — one
  birch per cmux workspace; `birch-cmux socket` prints it for reverse-reveal
  (`birch-ctl --socket "$(birch-cmux socket)" reveal <file>`). Implementation note:
  keyed on the UUID's first segment — the full UUID pushes the path past the
  ~104-byte unix-socket limit under macOS's `$TMPDIR`.

Documented in `docs/integrations.md` (new cmux section) and the README's integration
list. Live verification in the maintainer's cmux session is part of this sprint's
gates.
