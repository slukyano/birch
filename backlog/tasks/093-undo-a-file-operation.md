---
type: Task
title: Undo a file operation
description: Sprint 021 makes birch mutate the filesystem; trash covers delete and nothing covers rename, move, or create.
status: Draft
priority: medium
---

Until sprint 021, a wrong answer from birch was wrong on screen. After it, birch renames, creates,
moves, and deletes — and **only delete has a way back**, through the platform trash
([ADR 0031](../../docs/adr/0031-delete-goes-to-trash.md)). A rename typed in haste, a drag that
landed one row off ([ADR 0032](../../docs/adr/0032-move-by-dragging.md)), or a file created in the
wrong directory all leave the user to reverse it by hand, having first worked out what happened.

`docs/design.md` says *"no op-history/undo stack"* in the File operations section. Since
[ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md) that is the section's own decision
rather than a scope fence, so it is arguable on its merits — exactly as "no move" was, and that one
did not survive contact with a maintainer request either.

## What a design has to answer

- **How far back, and across what.** A **session-only** stack is cheap and covers the mistake people
  actually make — the one noticed immediately. Undo *across restarts* means a journal of filesystem
  mutations on disk, which ADR 0031 already argued is a different product. Deciding this first
  decides most of the rest.
- **Delete is the asymmetric one.** Its undo lives outside birch, in the trash. Restoring from the
  trash portably is not something the `trash` crate offers everywhere, so delete may be the one
  operation undo cannot reach — which would make "undo" mean *all operations except the destructive
  one*, an awkward thing to explain.
- **Undoing a create means deleting.** If the file has been written to since, undo destroys work that
  birch never saw. Trashing it instead of unlinking is the obvious mitigation and worth stating
  rather than assuming.
- **The world moves underneath.** Undoing a move needs the source still free and the destination
  still there; undoing a rename needs the old name still available. Every entry can be invalidated by
  the filesystem, another process, or the user. An undo that fails must say why, and must not
  half-apply.
- **`Ctrl-Z` is available, and that may be a trap.** birch runs in raw mode, where signal generation
  is off, so `Ctrl-Z` arrives as an ordinary key rather than suspending the process. The conventional
  undo key is therefore bindable — but a user who expects `Ctrl-Z` to background birch will find it
  does something else entirely, and losing suspend is a real cost in a terminal.
- **One step or many.** A single-level undo is far simpler and covers most of the value. A stack
  invites "redo", and then birch owns a history model.

## Related

- `029-add-file-operations` — the operations this would reverse.
- `072-drag-to-move` — the one whose mistakes are easiest to make, since a drag can miss by a row.
- ADR 0031 and ADR 0032 both record "there is no undo" as a consequence; whichever way this lands,
  they need amending.
