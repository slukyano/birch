---
type: Task
title: Add file operations
description: Rename/delete/new inline ops with git-aware delete - the rest of the 0.5 bundle split off to 071 (context menu) and 028 (copy paths).
status: Draft
priority: medium
blocked_by:
- 002-build-core-tree-view
---

Phase 0.5 of [the design doc](../../docs/design.md): exactly four ops (rename, delete-to-trash,
new file, new dir) as inline row editing, with git-aware delete confirmation. The other two thirds
of the 0.5 bundle are their own tasks — `071-add-context-menu` (the menu and hover highlight) and
`028-add-copy-paths` (copy name / relative path / absolute path over the OSC 52-first chain) — so
this task is the op layer alone: the mutations, and the watcher-event ownership that keeps the tree
from flickering or jumping mid-edit.

Compact-chain specifics from the design doc land here too: F2 on a chain inline-edits
the full `a/b/c` fragment (rename-with-path reused), and mouse segment-clicks target
individual chain members so a menu or op scopes to the clicked dir.

Design-phase open question (from the design doc): trash on exotic filesystems / NFS —
fallback behavior when trash is unavailable.

## Design

This is where birch starts writing to the filesystem. Everything before it read the tree, painted
it, and reported paths; a wrong answer was wrong on screen. From here a wrong answer is wrong on
disk, and the design is shaped around that rather than around the four operations, which are
individually small.

Three things need deciding, and each gets an ADR: **how text is entered** (birch has no modes and
this needs one), **what makes a delete safe** (trash, git, and where confirmation is left), and
**how a mutation and the watcher agree** about a tree that is changing underneath both.

### The inline editor, and the mode problem

`docs/design.md` chose inline row editing over a prompt bar — *"never a bottom command bar; prompt
bars grow verbs"* — and that choice creates birch's first modal state. While a name is being typed,
printable characters must reach the editor rather than the search, which is precisely what
[ADR 0008](../../docs/adr/0008-q-types-into-search.md) says they never do.

**The reconciliation is that ADR 0008 was never about search. It is about there being no letter
*commands*.** Printables go to the focused text field; the tree's field is the search box, and an
edit temporarily focuses a different one. Nothing becomes a hotkey, which is the property that rule
exists to protect. Stated that way the rule survives intact and the editor is not an exception to
it — [ADR 0030](../../docs/adr/0030-the-inline-editor-is-the-only-mode.md) records the reading.

It is still a mode, and the identity refuses modes ("*Not broot — no verbs, no modal behavior*").
What makes this one admissible is that it is **self-evident, transient, and singly-exited**: a text
cursor sits in a row you are looking at, `Enter` commits, `Esc` cancels, and there is no way to be
in it without seeing that you are. A vim mode is none of those things. The design will not admit a
second mode on this precedent.

**`Esc` gains its third layer**: edit → menu → search → quit. The menu cannot be open during an edit
— it closes when it invokes — so the layers never contend.

**The edit is a paint-time overlay, not a tree mutation.** A rename draws over the existing row; a
new file draws a **phantom row** under the target directory. Neither touches `Tree` until `Enter`.
That keeps the real-tree/render boundary that `AGENTS.md` names load-bearing, and it means an edit
in flight cannot leave a half-made node behind if birch dies.

### Delete: trash first, confirmation only where nothing else helps

`docs/design.md`'s rule is *undoable beats confirmable*. The refinement this design adds is that
there are **two** safety nets, not one, and the friction goes where neither reaches:

| situation | what happens |
|---|---|
| trash works, every target tracked in git | **deletes immediately.** Recoverable twice over; a prompt would be theatre. The status line says what went. |
| trash works, any target untracked | **confirms once**, naming the count and the reason — an untracked file exists nowhere else, and a trash can be emptied. |
| trash unavailable (NFS, exotic filesystem, no XDG trash dir) | **confirms**, and says plainly that the delete is permanent. Declining does nothing. |

**The confirmation is `Enter` / `Esc`, never `y`/`n`.** A letter key is exactly what ADR 0008
forbids, and `Enter` already means commit while `Esc` already means back out. The prompt lives on
the status line, which is a *message*, not the input surface the design doc rules out.

**Trash comes from the `trash` crate.** Doing it properly is not a `rename` into a folder: macOS
records put-back information, and the XDG spec wants `.trashinfo` files and a per-mount trash
directory with its own collision rules. On Linux the crate is plain `std::fs` over that spec; on
macOS it is a thin objc binding — materially unlike the X11/Wayland linkage that made an in-process
clipboard unattractive in [`028`](028-add-copy-paths.md).

[ADR 0031](../../docs/adr/0031-delete-goes-to-trash.md) records this.

### Arity, now that a selection can hold several rows

[`074`](../archive/074-add-multi-selection.md) deliberately left this per operation. Settled here:

| action | targets | why |
|---|---|---|
| **Delete** | **one or many** | The natural plural operation, and the one trash makes safe. This is the first action to act on a whole selection. |
| **Rename** | exactly one | One row is being edited; a plural rename is a different feature with a different interface. |
| **New File / New Dir** | exactly one | The target is a *directory*, and several selected rows do not name one. |

**All four are unavailable in `--pick`.** Mutations are human-initiated and disabled in the picker
(`docs/design.md`), which is the first real use of `ActionContext::picker` — a field
[ADR 0029](../../docs/adr/0029-the-menu-projects-the-action-layer.md) defined and nothing had yet
consulted.

### The watcher, and why ownership is smaller than it sounds

The task inherited a worry about "watcher-event ownership… no flicker, no selection jumps mid-edit".
The architecture already answers most of it. The watcher does not emit per-file events; it emits
`Dirty(dirs)`, and a dirty directory is re-listed into a `Snapshot`, which **reconciles** rather than
appends ([ADR 0006](../../docs/adr/0006-snapshot-deltas-stateless-sources.md)). So an operation does
not need to suppress anything: it applies its own delta immediately, and the snapshot that follows
converges on the same state.

What genuinely needs protecting is the **edit in flight**. Deltas keep arriving while a name is being
typed, and rows reshuffle as they do. Because the editor is a paint-time overlay keyed on a *path*
(rename) or a *parent plus position* (new), a reshuffle moves the row and the overlay with it; the
text being typed lives in the editor, not in the tree, so nothing can disturb it. A phantom row is
pinned under its parent and stays there until the edit ends, whatever the listings do.

### The four operations

- **Rename — `F2`.** The row becomes a text input containing its current name, cursor at the
  extension boundary. Editing to a path (`../other/name.ts`) performs a **move**, which is the 80% of
  move without the complexity cliff; the parent must exist. On a compacted chain the editor holds the
  full `a/b/c` fragment, since that is what the row shows.
- **New File — `Ctrl-N`.** A phantom row under the target directory. `foo/bar/baz.ts` creates
  intermediate directories; a trailing slash (`foo/`) creates a directory instead. One binding, three
  operations, and no dedicated new-dir key. On a compacted chain it creates in the tail.
- **New Dir** — the trailing-slash form of the above; not separately bound.
- **Delete — `Delete`.** Per the table above.

**`Delete` on an Apple keyboard is `Fn`-`Delete`**, because the key labelled *Delete* there sends
Backspace — which birch already gives to the search. Binding Backspace to a destructive action on a
key whose meaning depends on whether a search is open is exactly the kind of contextual hazard this
design should not create, so it is not done. The menu entry is the discoverable path, and the sharp
edge is documented rather than engineered around.

### Public-surface delta

| surface | change |
|---|---|
| key bindings | **`F2`** rename · **`Ctrl-N`** new file/dir · **`Delete`** delete · `Enter`/`Esc` gain meanings inside an edit and a confirmation; `Esc` gains a third layer |
| menu | four new entries: New File…, Rename…, Delete (and New Dir via the trailing slash, not a separate entry) |
| CLI flags · config keys · environment variables | none |
| socket / protocol | **none** — mutations are human-initiated, and `docs/design.md`'s "no mutation verbs" stands |
| on-disk | **birch now writes to the filesystem**: creates, renames, moves, and trashes paths under the root |
| library API | `birch_core::action` gains four variants; a new `birch_core::ops` module |
| dependencies | **`trash`** — new |

### Implementation

- **`crates/birch-core/src/ops.rs`** — the four operations as functions over real paths, returning
  the `TreeDelta`s they imply. No ratatui, no view.
- **`crates/birch-core/src/action.rs`** — the four variants, their availability (arity per the table,
  and all four false when `picker`).
- **`crates/birch-tui/src/edit.rs`** — the editor model: buffer, cursor, and the two anchoring modes
  (over a row, or phantom under a parent). Pure logic, no terminal.
- **`crates/birch-tui/src/render.rs`** — painting the editor over a row or as a phantom, reusing the
  row geometry so an edit does not shift the tree.
- **`crates/birch-tui/src/input.rs`** — `F2`, `Ctrl-N`, `Delete`, and routing printables to the
  editor when one is open.
- **`crates/birch/src/app.rs`** — the edit and confirmation states, the `Esc` layer, `perform` arms,
  and applying each operation's deltas.

### Tests and verification

Unit: each operation over a temporary directory, including the intermediate-directory create, the
trailing-slash directory, and rename-to-a-path performing a move; the availability table, including
every action refusing in `--pick`; the editor's buffer and cursor; the confirmation matrix — tracked
deletes immediately, untracked confirms, trash-unavailable confirms and says permanent; a delta
arriving mid-edit moving the overlay without disturbing the text. Hands-on: drive the binary through
each operation in a scratch directory and observe the filesystem afterwards, and record `vhs` frames
of an edit in flight and of the confirmation, since both are visual states that did not exist before.
