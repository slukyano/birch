---
type: Task
title: Make git badge placement configurable
description: Badges (M/U/A letters) become a theme axis and a setting — right column, left of the name, or none — with a fitting default per built-in theme.
status: Draft
priority: medium
blocked_by:
- 054-refine-tree-visual-design
---

Maintainer request: the right-side git status badges become configurable in the theme, plus a
setting. Options: **right side**, **left side**, or **none** — with the appropriate choice picked
for each built-in theme.

## Surface

- **Theme axis**: `BadgePlacement { Right, Left, None }` alongside the existing `BadgeStyle`
  (`Letter` / `Symbol`), so a theme states both *where* and *how* status is drawn.
- **Setting**: overridable at runtime and persistently — a `badges` key in the config file, a CLI
  flag, and a `birch ctl set badges right|left|none` key (the protocol's `set` is additive).
- **Per-theme defaults**, grounded in the measured originals
  ([editor ground truth](../../docs/research/editor-trees.md)):
  - `xcode` → **right** (Xcode's navigator has exactly this right-aligned `A`/`M` column);
  - `vscode` → **right** (the explorer's letter badge sits at the row's right edge);
  - `jetbrains` → **none** — IDEA carries VCS state in the *filename colour*, no letter;
  - `retro` (the Commander) → **none** or right; NC had no per-file VCS column, and the palette
    already colours names — design picks;
  - `birch`, the scheme themes → **right** (today's behaviour).

## Notes

- `none` is not the same as `--no-git`: git stays on, names keep their status colours and folders
  keep their rollups — only the badge column disappears. Both must keep working independently.
- **Left placement shifts the row layout**, so `hit_test` geometry must move in lockstep with the
  painted columns (the chevron zone is `depth * INDENT_WIDTH` today). This is the one part of the
  task that is not purely cosmetic; cover it with tests like the folder-style layouts.
- The rollup `●` for directories follows the same placement.
- `right` currently reserves a fixed 2-column gutter; `none` should give that width back to names.

Related: `061` (theme axes), ADR 0021 (the theme system).

## Design

Delivered as specified above — `BadgePlacement { Right, Left, None }` as a theme axis beside
`BadgeStyle`, a `badges` key across config, flag, and `ctl set`, and per-theme defaults. What the
design phase settled is how it sits beside the two neighbours that arrived since:

- **The gutter is two columns wide but paints one.** `BADGE_WIDTH` is 2 and `badge_line` emits a
  single glyph, so the second column is blank padding. Placement moves the gutter as a whole; it
  does not change that width, which matters because [`078`](078-add-branch-diff-badges.md) claims
  the spare column.
- **`Left` places the gutter between the indent guides and the name**, so the name's start column
  shifts by the gutter width. `hit_test` computes the chevron zone from `depth * INDENT_WIDTH`
  today; under `Left` both the chevron zone and the name zone move, and the two must be derived
  from one shared geometry rather than computed twice.
- **`None` returns the width to names** and keeps everything else: status colours on names, rollup
  dots on directories, and `--no-git` remaining a separate switch that turns the *data* off.
- **The scrollbar is outside the gutter.** [`068`](../archive/068-add-scrollbar.md) took the
  far-right column and reserves it only while shown; under `Right` the order is
  `tree │ gutter │ badges │ bar`, and under `Left` the bar keeps its column while the badges move
  to the other side.

### Public-surface delta

- **CLI**: `--badges <right|left|none>`.
- **Config**: `badges = "right"`.
- **Socket**: `SettingKey::Badges`, value parsed like `Theme` rather than as on/off — additive.
- **Theme**: a `BadgePlacement` axis, with per-theme defaults as tabulated above.
