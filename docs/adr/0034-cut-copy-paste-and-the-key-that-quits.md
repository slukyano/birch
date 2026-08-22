---
type: ADR
title: Cut, copy and paste — and Ctrl-C stops quitting
status: Proposed
date: 2026-08-22
supersedes: []
amends: [0032]
---

# Context

Move had two routes and both fail the same case. A **drag** ([ADR 0032](0032-move-by-dragging.md))
needs source and destination on screen together; edge-scrolling helps, but dragging across a long
scroll is miserable. **Rename-to-a-path** means typing `../../other/place/name.ts` by hand, and
nothing on screen suggests the field accepts it. Neither answers *move this to somewhere far away*,
which is the ordinary case a file manager is for.

`docs/design.md` also said **"No copy"** — a prohibition the File operations section stated for
itself, which [ADR 0028](0028-the-identity-is-the-scope-fence.md) makes arguable on its merits rather
than a scope fence.

And the conventional key was taken: **`Ctrl-C` quit birch**.

# Decision

1. **Cut (`Ctrl-X`), Copy (`Ctrl-C`), Paste (`Ctrl-V`)**, with menu rows for all three. One slot,
   session-only; holding a new selection replaces whatever was held, because a clipboard with more
   than one slot is one nobody can keep track of.
2. **`Ctrl-C` no longer quits. Quit is `Esc` and `Ctrl-Q`.** This is the expensive half of the
   decision and it is taken deliberately. `Ctrl-C` is the most reflexive key in a terminal — but it
   is also the only key anybody reaches for to copy, `Ctrl-Shift-C` cannot be represented at all in
   the legacy encoding (task 091), and a copy nobody can reach is not a feature. Raw mode already
   clears `ISIG`, so `Ctrl-C` was never a signal inside birch: it was a binding, and bindings are
   ours to spend. `Ctrl-Q` is added so quit is never unreachable if `Esc` is busy backing out of a
   layer.
3. **Paste lands in the selected directory**, or in the selected file's parent — the same target rule
   a create already uses, so there is one answer to "where do new things go".
4. **A paste never overwrites and never silently renames**, which is ADR 0032's rule applied
   unchanged. A collision is refused and said out loud.
5. **A paste is one undo step**, however many rows it carried: a cut-paste records the moves, a
   copy-paste records what it created, and `Ctrl-Z` reverses the lot.
6. **Copying a symlink copies the link**, not what it points at. Following it would silently turn one
   entry into a whole tree, and could not be undone as one.

# Consequences

- **birch can duplicate files, which it could not before.** `docs/design.md`'s "No copy" is withdrawn.
  The capability follows from the gesture rather than the other way round: once there is a clipboard,
  a paste that duplicates is the same mechanism as a paste that moves, with one flag different.
- **An entry cannot be duplicated into its own directory.** Decision 4 refuses the collision, and
  there is no name for the copy that birch may invent — "a copy 2" is exactly the silent renaming the
  rule forbids. This is a real limitation, and the alternative is worse.
- **Muscle memory will cost people a birch session or two.** Someone who has quit with `Ctrl-C` for
  years will copy instead, and copying is harmless — which is the right direction for the mistake to
  fall. The reverse binding would have made a stray `Ctrl-C` destroy a session's state.
- **Move now exists three ways** — drag, rename-to-a-path, cut-paste. ADR 0032 already accepted two
  on the grounds that a gesture and a keyboard route are different things; a third that answers
  *distance* is the same argument again.
