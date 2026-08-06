---
type: Task
title: Select on mouse release, not on press
status: Designed
priority: medium
description: A click becomes a press and a release on the same row, taking effect on the release, so a click can be revoked by sliding off.
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

## Design

Recorded as **[ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md)**: a click is a press
and a release on the same row, and it takes effect on the release.

### Input layer

`InputAction::Click { column, row }` splits into two actions carrying the same coordinates:

| crossterm event | today | after |
|---|---|---|
| `Down(Left)` | `Click` — acts immediately | `Press` — arms only |
| `Up(Left)` | discarded | `Release` — completes or abandons |
| `ScrollUp` / `ScrollDown` | unchanged | unchanged (no release exists) |

`map_event` stays what it is — pure position mapping with no state. Every decision about what a
press *means* belongs to the app, which is where the rows are.

### App layer

One new piece of state: the armed press, `Option<{ path: PathBuf, on_chevron: bool }>`.

- **`Press`** hit-tests the coordinates and stores the row's real path with the zone it landed in.
  A press outside the tree stores nothing. Nothing else happens — no selection, no toggle, no
  timer.
- **`Release`** hit-tests again and completes the click **only** when the row path *and* the zone
  both match the arm. On a match, today's `resolve_click` runs unchanged, at release time. On any
  mismatch — a different row, the other zone, or no row at all — the press is abandoned and the
  pending double-click is disarmed, which is already how ADR 0015 treats an intervening click.
- The arm is cleared by every release, and replaced by every press, so a lost release cannot leave
  something that fires later.

Keying on the path rather than the index is what makes this safe under live updates: a snapshot
arriving between press and release can reorder rows, and the click still resolves to the row it
started on, or to nothing.

### Double-click

`ClickTimer::observe` moves from press time to release time, so its 450 ms window measures
**release to release** — two complete clicks, which is what ADR 0015 describes in words. The timer
itself is unchanged, including its path keying and the chevron's disarm.

### Dimmed rows

Unchanged from [ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md): a dim row is
inert, except that a chevron click still toggles a dim directory. The check simply runs at release.

### Public-surface delta

**None.** No new or changed CLI flag, config key, socket field, environment variable, or on-disk
path. The change is behavioural, and the one place it is documented — the Mouse section of
[`docs/design.md`](../../docs/design.md) — gains the release rule.

### Tests

The pure parts are testable without a terminal, which is the point of keeping `map_event` stateless:
`Down`/`Up` mapping; a press-then-release on the same row selecting; a release on a *different* row
doing nothing; a release outside the tree doing nothing and disarming a pending double; two
complete clicks activating while press-release-press does not; and a chevron press released on the
name not toggling.
