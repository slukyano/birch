---
type: Task
title: Hover highlight in the tree
description: The row under the pointer lights up. Motion (SGR 1003) is already reported; what is missing is a tree that acts on it.
status: Draft
priority: low
---

`docs/design.md` lists **hover highlight (SGR 1003 motion tracking)** among the mouse features. The
context menu acts on motion, because a menu that does not light the entry under the pointer cannot be
used with a mouse. Hover in the tree itself is what remains.

The motion events are **already arriving**: crossterm's `EnableMouseCapture` requests mode 1003 at
startup along with the rest, and birch cannot decline it without hand-rolling the whole mode set.
`071-add-context-menu` believed it was enabling 1003 for the menu's lifetime and disabling it after;
in fact the enable was a no-op and the disable dropped mouse reporting below the level the terminal
had been told to use, which is why every right-click after the first went to the terminal (sprint
021). So the wake-ups this task worried about are being paid for today, whatever it decides.

## What a design has to answer

- **The event cost, which is a *redraw* cost.** The wake-ups already happen; what hover would add is
  a draw per movement. [ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md) coalesces a
  batch into one draw, which is the mechanism that makes this affordable. Declining the events
  themselves is possible only by replacing `EnableMouseCapture` with a hand-written mode set — and
  the mode set is exactly where this went wrong once already, so it wants a measurement behind it.
- **Whether it is opt-out, opt-in, or automatic.** A setting is the obvious hedge, and settings have
  a cost of their own.
- **What hover looks like** next to the selection highlight and the cursor row, in both
  `SelectionStyle`s, without a third background colour reading as a third kind of state.
- **Dim rows.** A dimmed row is inert ([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md));
  highlighting one under the pointer advertises a target that does not answer.

## Related

- `071-add-context-menu` — the only consumer of motion today; this generalises it to the tree.
