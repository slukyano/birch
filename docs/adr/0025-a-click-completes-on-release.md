---
type: ADR
title: A click completes on release, on the row it started on
status: Proposed
sprint: sprint-017
---

# Context

[ADR 0015](0015-click-selects-double-click-activates.md) settled *what* a click does — single
selects, double activates, a chevron toggles — but not *when* it happens. The implementation acts
on button-down: `map_event` maps `MouseEventKind::Down(Left)` to a `Click` action and discards
`Up` entirely. Selection therefore lands the instant a button touches the mouse, before a drag or a
change of mind can be expressed, which reads as twitchy in live use. herdr acts on release, and
that feels deliberate by comparison.

The moment is about to become load-bearing beyond feel. `072-drag-to-move` must separate "clicked"
from "started dragging", and under mouse capture a drag *is* press → motion → release — the same
shape as a click. A model that commits at press has already acted before a drag can be recognised.

# Decision

**A click is a press and a release on the same row; it takes effect on the release.**

1. **The press arms, it does not act.** A left button-down records which row it landed on and
   whether it landed in the chevron zone. Nothing is selected, toggled, or opened.
2. **The release completes it** — but only if it lands on the *same row*, in the *same zone*, as
   the press. Then ADR 0015's rules apply unchanged: select, or toggle if the zone is the chevron,
   or activate if it completes a double-click.
3. **Anything else abandons the press.** Releasing on a different row, outside the tree, or on the
   opposite zone of the same row does nothing at all, and clears any pending double-click. This is
   the universal pointer convention — press, move away, release to cancel — and it is what makes a
   click revocable.
4. **The armed press is keyed on the row's real path**, not its visual index, exactly as ADR 0015
   keys double-clicks. A live tree update between press and release cannot redirect a click onto a
   row that moved under it.
5. **The double-click window measures release to release.** Two *complete* clicks within 450 ms
   activate — which is what ADR 0015 already said in words, and what timing from presses only
   approximated.
6. **One rule for every affordance.** The chevron does not keep button-down for immediacy. Two
   moments would mean a press on a chevron committed while a press on a name did not, and the
   difference between them is the few tens of milliseconds a real click takes.

Wheel events are unaffected — they have no release.

# Consequences

- **A click becomes revocable**, which is the point: pressing the wrong row and sliding off costs
  nothing. The cost is that birch now tracks one piece of pointer state it never had.
- **ADR 0015 stands except for its moment.** Single-click-selects, double-click-activates,
  chevron-toggles-immediately, and path-keyed detection all survive; "immediately" now means "on
  the release of that click" rather than "on its press". Everything else in that ADR is unchanged.
- **`072-drag-to-move` has somewhere to attach.** A drag threshold sits naturally between the armed
  press and the release, and no longer has to undo an action that already happened.
- **`071-add-context-menu` inherits the contract** for its right button, and its
  selection-coupling question is asked against a model where the press is only an arm.
- **A press with no release is inert.** Losing the release — focus leaving the terminal mid-click —
  leaves an arm that can only be completed by a matching release, and is replaced by the next
  press. No timeout is needed.
- **Terminal floor.** birch enables SGR mouse encoding (1006), where a release identifies its
  button. Should a terminal report an ambiguous release instead, the fallback is exact because at
  most one press is ever armed: any left-button release completes the armed press.

# Alternatives considered

- **Keep the chevron on press, move only selection to release.** Rejected per decision 6: two
  moments for two zones of the same row is a rule users would have to learn from feel alone, and
  the immediacy it buys is imperceptible.
- **Act on press, then undo on a drag-away.** Rejected: an action that visibly happens and then
  un-happens is worse than one that never happened, and selection changes are observable through
  the socket.
- **Complete the click wherever it is released** (ignore which row). Rejected: it makes a click
  unrevocable again, in a way that also silently retargets it.
- **A time threshold instead of a position one** (a long press is not a click). Rejected: it adds a
  timer to a gesture that does not need one, and would delay every selection by the threshold.
