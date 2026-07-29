---
type: Task
title: Select on mouse release, not on press
status: Draft
priority: medium
description: Clicks act on button-down today, which feels twitchy; herdr acts on release, which reads as deliberate.
---

Maintainer report: mouse selection feels wrong, and the likely reason is that it happens on
**button down**. herdr selects on **button up**, and that feels better.

`input::map_event` maps `MouseEventKind::Down(Left)` to `InputAction::Click`
(`crates/birch-tui/src/input.rs:89`); `MouseEventKind::Up` is discarded. So the selection lands the
instant a button touches the mouse, before a drag or a change of mind can be expressed.

## Direction

Act on **release**, keeping press only for what genuinely wants it. Questions for design:

- **Which affordance keeps press?** A chevron toggle arguably wants the immediate feel; a name
  selection wants the deliberate one. Splitting them is possible but two rules are a cost.
- **What happens between press and release?** A press that leaves the row before releasing should
  presumably do nothing. That requires tracking the press row, which birch does not do today.
- **Double-click detection** currently keys on press events (ADR 0015's `ClickTimer`, keyed on the
  row path). Moving to release changes what the 450 ms window measures; the timer must be re-based
  on releases so a double-click is still two complete clicks.
- **Scroll-wheel events** are unaffected (they have no release).

Touches [ADR 0015](../../docs/adr/0015-click-selects-double-click-activates.md) — the click model
itself, not just its implementation, so an amendment or superseding ADR is likely.
