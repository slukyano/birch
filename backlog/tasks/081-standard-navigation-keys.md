---
type: Task
title: Support the standard navigation keys
description: PgUp/PgDn/Home/End do nothing today - bind them, and decide whether a page moves the selection or only the viewport, and what they mean under a live search.
status: Draft
priority: medium
---

`PageUp`, `PageDown`, `Home`, and `End` are dead keys in birch. `map_event`
(`crates/birch-tui/src/input.rs:71`) maps `↑ ↓ → ← Enter Esc Backspace`, `Ctrl-C`, and printables;
everything else falls into `_ => None`. In a tree that can be thousands of rows long, the only way
to the bottom is holding `↓` or spinning the wheel — and the wheel moves the viewport without
moving the selection, so it does not get you anywhere you can act.

These four keys are cheap to bind and unusually safe: they are non-printable, so they do not touch
the reserved-character rule ([ADR 0008](../../docs/adr/0008-q-types-into-search.md)) the way `?`
does for `073-hotkey-reference`. Every list-shaped TUI and every editor file tree has them; their
absence reads as an omission rather than a choice.

## What the design has to settle

**Does a page move the selection, or the viewport?** birch keeps the two decoupled: `↑`/`↓` move
the selection (`view.move_by`), while a wheel scroll takes the count-only fast path and moves the
viewport alone ([ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md)). `PgUp`/`PgDn` are
keyboard navigation, which argues for moving the selection by a page and letting the viewport
follow — the editor-explorer behaviour. Whichever is chosen, `Home`/`End` should match it.

**How big is a page?** `render::tree_viewport_height` is the obvious unit. Decide whether it is the
full height or height minus one row of overlap (the usual reading-continuity trick), and whether a
page that runs past the end clamps to the last row rather than stopping short.

**What do they do under a live search?** This is the sharp one. With a live search, `↑`/`↓` cycle
*matches* and non-matching rows are dimmed and inert
([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)). So a page either steps a
page's worth of *matches* (consistent with `↑`/`↓`, but "a page" then means nothing visual), or
moves a page down the rendered rows and lands on the nearest selectable one. `Home`/`End` have the
same fork: first/last row, or first/last match.

**Scope of `Home`/`End`.** Whether `Home` lands on the root row or the first child under it, and
whether `Ctrl-Home`/`Ctrl-End` need to exist at all as separate bindings. `Shift`-modified variants
are out — range selection belongs to `074-add-multi-selection`, which is itself scope-fenced.

**`--pick` mode** inherits all of it: the picker is the same tree and the same key handling, and a
long picker list is exactly where paging is worth the most.

## Surfaces it touches

New `InputAction` variants and their arms in `map_event`, the dispatch in
`crates/birch/src/app.rs` (`handle_input`), and the binding tables that document keys — `--help`,
the README, and `docs/design.md`. `073-hotkey-reference` will list whatever this task binds, so the
two should agree on names.
