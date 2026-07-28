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
