---
type: ADR
title: Undo is a session stack, and delete is the step it cannot take back
status: Proposed
date: 2026-08-22
supersedes: []
amends: [0031, 0032]
---

# Context

Sprint 021 made birch write to the filesystem — rename, create, move, delete. Only delete had a
way back, through the platform trash ([ADR 0031](0031-delete-goes-to-trash.md)), and even that one
lives outside birch. A rename typed in haste or a drag that landed one row off left the user to
reverse it by hand, having first worked out what happened. `docs/design.md` said *"no
op-history/undo stack"*; since [ADR 0028](0028-the-identity-is-the-scope-fence.md) that is the
section's own decision rather than a scope fence, so it is arguable on its merits, and the
maintainer asked for it.

Three facts constrain the shape, and each was measured rather than assumed:

1. **The `trash` crate cannot restore on macOS — but the OS can.** `os_limited::restore_all` is
   compiled only for Windows and Freedesktop systems (`trash-5.2.6/src/lib.rs:345`), and the crate's
   macOS delete calls `-[NSFileManager trashItemAtURL:resultingItemURL:error:]` passing **null** for
   the out-parameter, discarding the one thing a restore needs: where the item landed. birch makes
   the same call and keeps the answer, so a restore is a rename back. The OS still performs the
   move, so decision 4 of [ADR 0031](0031-delete-goes-to-trash.md) — do not hand-roll a trash —
   stands untouched.
2. **`Ctrl-Z` is free.** Raw mode runs `cfmakeraw`, which clears `ISIG`, so `Ctrl-Z` has never
   suspended birch and never could. `093-undo-a-file-operation` worried that binding it would cost
   the user suspend; it costs nothing. It is a control character, so [ADR 0008](0008-q-types-into-search.md)
   is untouched.
3. **Undoing a create means deleting**, and the file may have been written to since birch made it.

# Decision

1. **A session-only stack, bounded at 100 steps.** Undo across restarts means a journal of
   filesystem mutations on disk, which ADR 0031 already argued is a different product. The mistake
   people actually make is the one they notice immediately.
2. **Every step is validated when it runs, never when it is recorded.** The world moves underneath:
   the old name may be taken again, the file may be gone, another process may have moved it. A step
   that cannot be reversed says why and reverses nothing. A batch stops at the first refusal and
   reports what it already did — a partial undo is reported, never silent.
3. **A refused step is still consumed.** A step that stayed on the stack after refusing would refuse
   again on the next `Ctrl-Z`, and undo would be permanently stuck on it.
4. **Undoing a create trashes what it made**, rather than unlinking it, for the same reason
   ADR 0031 sends deletes to the trash. It takes back the *shallowest* path the create announced:
   `a/b/c.txt` typed into an empty tree made `a`, and removing only `c.txt` would leave the rest.
5. **A delete is reversed by restoring the files**, not by telling the user where to find them. A
   removal records what each path takes to come back — the item's location in the trash on macOS,
   the platform's own trash record elsewhere — and `Ctrl-Z` puts it back. A restore **refuses rather
   than overwrites**: the old name may have been taken while the file sat in the trash. Where the
   platform reports nothing to restore from, the step says so plainly instead of guessing at a file
   in the trash with a matching name.
6. **No redo.** A stack that invites redo is a history model, and birch does not own one.

# Consequences

- **Undo covers every operation birch performs**, delete included. Reaching that took going around
  the `trash` crate on macOS rather than accepting its limits, which is the general lesson: a
  dependency's API surface is not the platform's.
- **ADR 0031 and ADR 0032 both record "there is no undo" as a consequence.** Both are amended: the
  statement now holds only for delete.
- **The menu row names what it would reverse** (`Undo Rename of b.txt`), so it can be trusted
  without remembering what was just done. It is the only action whose availability ignores the
  selection entirely.
- **The stack is one more thing an operation must remember to feed.** Every mutation pushes a step
  at its success path; one that forgets is an operation undo silently skips past. The alternative —
  deriving history from the delta stream — would catch watcher events too, and undoing another
  process's work is not something birch may do.
