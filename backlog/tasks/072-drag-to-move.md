---
type: Task
title: Move files and folders by dragging
description: Drag a row onto a directory to move it - requested by the maintainer; currently forbidden by the scope fence, so it needs a fence amendment before design.
status: Draft
priority: medium
---

Maintainer request: drag a row onto a directory row and drop it there to move the file or folder,
the way Finder and the IDE explorers do.

## Scope-fence conflict (decide first)

[The design doc](../../docs/design.md) forbids this three times over, and the fence is binding:

- **"Permanently out of scope: … drag-and-drop move"** (the identity section's whitelist);
- **"No drag-and-drop"** (the mouse section);
- **"Exactly four: rename, delete, new file, new dir. No move (see below), no copy"** (file
  operations), where the rationale is that *rename-to-a-path* already covers move: editing a row
  inline to `../other/name.ts` performs the move, called "the 80% of move without the complexity
  cliff".

So this task cannot be designed as written until the fence moves. Amending it is a maintainer
decision and warrants an ADR — recording what the fence now says, and why the 80% path stopped
being enough (dragging is the discoverable gesture; inline rename-to-path is invisible to anyone
who has not read the docs).

## What the feature is

- Press on a row, move past a threshold, drop on a directory row → the path moves into it.
- Drop on a file row targets its parent directory; drop below the tree targets the root.
- Visible feedback: the dragged row's name follows the cursor (or the row lifts/dims), the drop
  target directory highlights, and an illegal drop reads as illegal before the button is released.

## The hard parts

- **Drag vs. click.** Under mouse capture a drag *is* press → motion → release, which is exactly
  the click model's shape. [ADR 0015](../../docs/adr/0015-click-selects-double-click-activates.md) acts on
  button-down and `067-select-on-mouse-up` proposes moving that to release; whichever wins, a drag
  threshold (cells moved, or a hold delay) has to separate "clicked" from "started dragging"
  without making ordinary clicks feel sticky.
- **Shift-drag is already taken.** Mouse capture disables native text selection, and the documented
  mitigation is that Shift-drag passes through to the terminal. That escape hatch must survive.
- **Auto-scroll and spring-loading.** A move usually crosses more tree than one screen shows:
  dragging to the pane edge should scroll, and hovering a collapsed directory should spring it open
  (Finder's behaviour) — which collides with **"never auto-expand"**, a standing rule motivated by
  ignored directories. A hover-expand is human-initiated, but the rule deserves an explicit carve-out
  rather than a quiet exception.
- **Illegal and colliding drops.** A directory into its own descendant; a drop where the name
  already exists (refuse, or offer a rename — no overwrite either way); a read-only destination;
  a compacted chain, where the drop target is a *segment*, not the visible row.
- **Undo.** There is deliberately no op-history stack, and trash cannot help a move. The reverse of
  a move is another move, which means the status line has to say clearly what happened and where it
  went.
- **Cross-instance and cross-root drags are not in this.** With `026-add-multiple-roots`, dragging
  between sibling roots is the same in-process gesture; dragging *between terminal panes* is not
  something a TUI can receive, and should be stated as a non-goal so it is not read as missing.

## Related

- `029-add-file-operations` — the op layer this rides on (watcher-event ownership during mutation,
  no flicker, no selection jumps).
- `071-add-context-menu` — a "Move to…" menu entry is the keyboard-reachable twin of the gesture,
  and may be the cheaper half of the same product goal.
- `074-add-multi-selection` — dragging a multi-selection is the obvious next ask; the two share a
  fence amendment.
