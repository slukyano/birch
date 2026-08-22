---
type: Task
title: Hover highlight in the tree
description: Motion tracking (SGR 1003) so the row under the pointer lights up; split out of the context menu, which enables motion only for its own lifetime.
status: Draft
priority: low
---

`docs/design.md:124` lists **hover highlight (SGR 1003 motion tracking)** among the mouse features.
`071-add-context-menu` enables motion tracking **only while the menu is open** — a menu without hover
cannot be used with a mouse — and switches it off again when the menu closes, so the tree never pays
for the event volume. Hover in the tree itself is what remains.

## What a design has to answer

- **The event cost.** Mode 1003 reports every pointer movement over the pane, at whatever rate the
  terminal samples. [ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md) coalesces a
  batch into one draw, which is the mechanism that makes this affordable, but the wake-ups still
  arrive — a birch idling in a side pane would burn cycles whenever the pointer crosses it.
- **Whether it is opt-out, opt-in, or automatic.** A setting is the obvious hedge, and settings have
  a cost of their own.
- **What hover looks like** next to the selection highlight and the cursor row, in both
  `SelectionStyle`s, without a third background colour reading as a third kind of state.
- **Dim rows.** A dimmed row is inert ([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md));
  highlighting one under the pointer advertises a target that does not answer.

## Related

- `071-add-context-menu` — enables motion tracking for the menu's lifetime; this generalises it.
