---
type: ADR
title: Files move by dragging; rename-to-a-path stays as the keyboard route
status: Accepted
sprint: sprint-021
---

# Context

`docs/design.md`'s File operations section fixed the set at four — rename, delete, new file, new
dir — and said **no move**, on a specific and good argument: editing a row's name to a path
(`../other/name.ts`) already performs a move, which is *"the 80% of move without the complexity
cliff"*.

Until [ADR 0028](0028-the-identity-is-the-scope-fence.md) that was also a scope-fence entry, and
therefore not arguable. ADR 0028 deleted the permanent list, leaving this as the section's own
decision — which means it can be revisited on its merits, and has to be, because the maintainer asked
for dragging.

Two things have changed since the original argument was made. A selection can now hold several rows
([`074`](../../backlog/archive/074-add-multi-selection.md)), and rename-to-a-path moves exactly one.
And the click model settled ([ADR 0025](0025-a-click-completes-on-release.md)), which is what makes a
drag separable from a click without a heuristic.

# Decision

**Dragging a row onto a directory moves it. Rename-to-a-path stays, unchanged, as the keyboard
route.**

The original argument was right about capability and wrong about discovery. Rename-to-a-path does
perform a move — and **nothing on screen suggests it**. A user editing a name has no reason to
believe the field accepts `../`, and the feature is therefore reachable only by having read the
documentation. Dragging is the gesture people try first, in every file manager and every explorer
panel the identity names.

1. **A drag is a click that left its row.** No threshold, no timer: the drag begins when the pointer
   leaves the row the press started on, which ADR 0025's arm-and-complete model already tracks. A
   click that wobbles within its row stays a click.
2. **The dragged set is the selection, by the menu's rule.** A press inside the selection drags all
   of it; a press outside collapses to that row first.
3. **Refusals are visible before release** — into a descendant, onto a colliding name, onto the
   source's own parent. A drop never overwrites, and never silently renames.
4. **Spring-loading needs no exception.** The "never auto-expand" rule is scoped to *ignored*
   directories, and search already auto-expands. A normal directory opens on hover; an ignored one
   stays shut while remaining a legal target.
5. **There is no undo**, here or anywhere in birch. The status line reports what moved and where.

# Consequences

- **Move exists twice, deliberately** — a gesture and a keyboard route. That is the point rather than
  a redundancy: the gesture is discoverable and the keyboard route is precise, and neither serves the
  other's user.
- **`docs/design.md`'s "Exactly four" is now five**, and its File operations section says so.
- **A destructive-ish operation is now one gesture away**, with no confirmation, and a slip lands a
  file in the wrong directory. The mitigations are that refusals are visible before release, that a
  move never overwrites, and that the status line says where things went — not a prompt, which would
  make the gesture useless.
- **Dragging is unavailable in `--pick`**, with every other mutation.
- **Cross-pane dragging is a non-goal**, stated so it does not read as missing: a TUI cannot receive
  a drag from another terminal pane.

# Alternatives considered

- **Keep "no move", rely on rename-to-a-path.** Rejected: it is undiscoverable, and it cannot move a
  multi-selection at all.
- **A "Move to…" menu entry instead**, opening a directory picker. Not rejected on merit — it is the
  keyboard-reachable twin of the gesture and may still be worth having — but it is a different
  feature with its own interface, and it does not answer the request.
- **A drag threshold in cells or milliseconds.** Rejected: every threshold is wrong for someone, and
  "left the row" is both free and exactly the granularity the tree operates at.
- **Offer a rename on a colliding drop**, reusing `029`'s inline editor. Deferred rather than
  rejected: a drop that sometimes opens a text field is a surprising drop, and refusing is the
  behaviour that can be relaxed later without breaking anyone.
