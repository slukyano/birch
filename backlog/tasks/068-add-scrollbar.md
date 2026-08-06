---
type: Task
title: Add a scrollbar
status: Designed
priority: medium
description: A one-column indicator at the pane's right edge, shown only when the rows overflow, with the usual flag/config/ctl toggle.
---

Maintainer request: birch should show a scrollbar.

## Shape

- **Hidden when everything fits** — a full-height bar carries no information and steals a column.
- **A way to turn it off**: a flag, and/or a config key, and/or a theme axis. Design picks; the
  cheapest coherent option is a setting alongside the other toggles, since it is a preference rather
  than a per-invocation choice.
- **Where it goes**: the right edge is conventional, but the right two columns are the git badge
  gutter today (`render::BADGE_WIDTH`), so the layout has to give it a column or share one.
- **What it is made of**: a block-glyph track (`│`/`█`, or the eighth-block ramp for sub-row
  precision), themed like the guides.

## Notes

- `hit_test` geometry must account for whatever column it takes, exactly as `064`'s left-side badge
  placement would.
- Dragging the bar is a separate, larger question (it needs mouse-drag tracking, which birch has
  never had); the first version can be a pure indicator.
- Interacts with `064` (badge placement) — both compete for the right-hand columns, so whichever
  lands second inherits the layout the first one leaves.

## Design

A pure indicator: it reports where the viewport sits and never accepts a gesture. Dragging it needs
mouse-drag tracking birch has never had, which arrives with `072`.

### Where it goes

The pane's right-hand furniture is `tree │ gutter(1) │ badges(2)` today (`render::draw`). The bar
takes **one column at the far right**, pushing the badges left and leaving the gutter — the column
that keeps truncated text off the badges — where it is:

```
before   [ tree ......................... ][ ][B B]
after    [ tree ........................ ][ ][B B][│]
```

The tree area absorbs the column, and only while the bar is shown. Reserving it permanently was
rejected: in a 30-column pane a blank column is real estate paid for nothing, and the appearing and
disappearing happens exactly when the row count crosses the viewport — a state change the user
caused, not a flicker.

### When it shows

All three must hold: `settings.scrollbar` is on, `rows.len() > viewport`, and the pane is wide
enough that the tree keeps at least one column after the furniture. A tree that fits shows nothing,
per the request — a full-height bar carries no information.

### What it looks like

A one-column track of the guide glyph in the guide colour, with a solid thumb in the accent colour
— the same visual family as the indent guides, so it reads as furniture rather than content. No new
theme axis in this version; if the bar needs to be styled apart from the guides, that belongs with
`064`'s theme work.

Thumb geometry, from `view.scroll`, `rows.len()`, and `viewport`:

- **length** = `max(1, viewport² / rows.len())` — proportional, never invisible;
- **position** = `scroll × (viewport − length) / (rows.len() − viewport)`, and clamped so the thumb
  touches the top **only** at `scroll == 0` and the bottom **only** at the maximum scroll. "Am I
  actually at the end?" is the question a scrollbar exists to answer, and rounding must not lie
  about it.

Whole-cell resolution is enough at one column wide; the eighth-block ramp is a refinement to reach
for only if the thumb reads as jumpy in a tall pane.

### Hit-testing

While the bar is visible its column is **inert** — `hit_test` returns `None` there rather than
selecting the row behind it. A pure indicator must not double as a selection surface, and this
reserves the gesture for drag-to-scroll later without changing behaviour twice.

### Public-surface delta

Complete list, matching every other visual toggle (`icons`, `git`, `compact`):

- **CLI**: `--no-scrollbar` (with the hidden bidirectional `--scrollbar`, as the others have).
- **Config** (`birch.toml`): `scrollbar = <bool>`, default **on**.
- **Socket protocol**: `SettingKey::Scrollbar`, serialized `scrollbar`, an on/off/toggle
  `SettingValue` like its neighbours — additive only.
- **`birch ctl set scrollbar <on|off|toggle>`**.

`Settings` gains `scrollbar: bool`. The three documented settings lists — the `docs/design.md`
Defaults table, the README `birch.toml` block, and `--help` — each gain the row.

### Ordering with the neighbours

`064` (badge placement) and `070` (filter match counts) both want the right-hand columns. This
lands first and defines the furniture order; whichever comes later inherits this layout rather than
negotiating with it.

### Tests

Geometry is pure and testable without a terminal: hidden when the rows fit; thumb length at extreme
ratios never zero and never longer than the viewport; the thumb touches top and bottom only at the
scroll extremes; `hit_test` returns `None` on the bar column while visible and the row when hidden;
and the layout arithmetic holds in a pane too narrow for the furniture.
