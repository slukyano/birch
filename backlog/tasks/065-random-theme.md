---
type: Task
title: Add a "random" theme that picks one of the others
description: --theme random / theme = "random" resolves to a randomly chosen built-in theme at launch.
status: Draft
priority: low
blocked_by:
- 025-add-visual-styles
---

Maintainer request: add a theme `random` that picks from all the other themes.

## Surface

`random` becomes a `ThemeId` value like any other — accepted by `--theme random`, the config
`theme = "random"`, and `birch ctl set theme random` — and resolves to a concrete built-in theme
when applied.

## Design questions

- **When it rolls**: once per launch (stable for the session — the recommended reading of "pick a
  theme") or on every `ctl set theme random`? A running instance re-rolling on each redraw would be
  unusable; the roll must happen at resolution time, not in `Theme::for_id`, which is a pure
  function called every frame.
- **The pool**: all themes, or all *except* the fallbacks (`plain`) and any theme that paints its
  own canvas (`retro`'s DOS blue is a striking thing to land on unannounced)? Suggested: all
  themes, since the point is surprise, with the decision recorded.
- **Discoverability**: the status line could name the theme that was rolled, so a good roll can be
  kept — otherwise the user has no way to learn what they got. Worth a `birch ctl get-theme` (or
  extending an existing getter) so the choice is scriptable too.
- Persisting `random` in the config must store `random`, not the rolled result.

## Implementation note

No new dependency is needed for the roll: seeding from `SystemTime::now()` nanoseconds or hashing
with `std::collections::hash_map::RandomState` is plenty for picking one of eleven values. Adding
the `rand` crate for this would be disproportionate.
