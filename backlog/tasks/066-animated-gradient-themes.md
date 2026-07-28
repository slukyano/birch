---
type: Task
title: Support animated gradient colours in themes
description: A theme can define a moving colour gradient (RGB-keyboard style) — start with a rainbow travelling left to right.
status: Draft
priority: low
blocked_by:
- 054-refine-tree-visual-design
---

Maintainer request: support dynamic gradient colours — a theme defines a set of colours that move,
like an RGB keyboard or mouse. Start with rainbow colours moving left to right.

## Shape

A theme gains a gradient definition: an ordered colour list, a direction (start with horizontal,
left → right), a speed, and the target it colours. Each cell's colour is a function of its column
and a time phase, so the band travels across the pane.

Design has to choose **what the gradient paints** — the whole row, names only, icons, the indent
guides, or the selection bar. Painting names hurts readability least when the gradient is
high-lightness and the background stays fixed; painting guides or the accent bar is the tamer,
more tasteful option and probably the right starting point.

## The load-bearing problem: birch has no frame clock

The render loop is **event-driven** — it wakes on input, filesystem, git, search, and socket events
and is otherwise completely idle. An animation needs periodic redraws, which turns an always-on
side pane into a process that never sleeps. This is the crux of the task, not the colour maths:

- gate it behind the theme (no gradient → no timer at all, today's behaviour preserved);
- keep the tick slow and configurable (10–15 fps is ample for a drifting gradient) and make the
  cost explicit in the docs;
- **pause when the pane loses focus**: crossterm supports terminal focus reporting
  (`FocusGained` / `FocusLost`), so an unfocused side pane can stop animating entirely — worth
  wiring as part of this task;
- also pause when nothing is visible (zero-height viewport) and while a child process owns the
  terminal.

## Notes

- Per-cell colour means per-character spans across a name, which multiplies span counts; measure
  before and after so the tick does not become the app's cost centre.
- Interacts with the catalog rule "semantics global, hues local": a gradient must not swallow the
  git status colours, or state stops being readable. Restrict it to non-semantic elements.
- Strictly opt-in, never the default, and never on `plain` (its whole point is ANSI safety).
- Accessibility: motion in a persistent side pane is fatiguing for some users; the setting must be
  easy to turn off, and it should default off even in themes that define a gradient.
