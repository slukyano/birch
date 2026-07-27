---
type: Task
title: Refine the tree's visual design — earn "beautiful"
description: Tree/indent guides, a curated palette, refined selection, breathing room. General render-layer polish, distinct from the choosable style presets (025).
status: Designed
priority: medium
---

The tagline promises "lean **and beautiful**"; this task earns the second word. It is the general
visual design of the tree — distinct from [`025-add-visual-styles`](025-add-visual-styles.md),
which is only about **choosable presets** (default / vscode / plain). This task is the baseline
aesthetic those presets build on.

All of it is **render-layer** (paint-time) polish — no new features, modes, or hotkeys; the
real-tree/render split holds. Keep it lean and tasteful: polish, not visual features.

Candidate work (the design phase picks and orders these):

- **Tree guides — the highest-ROI change.** Today it renders as an indented list with chevrons +
  icons; guides make it read as a *tree*. Either subtle VS Code-style indent guides (dim vertical
  lines, brightened along the selected row's ancestry) or classic connectors (`├──` / `└──` / `│`).
  Default to the subtle guides; the classic-connector look could be a `025` preset.
- **A curated palette** — a cohesive, muted palette (one accent for selection, calm git colors,
  tasteful or near-monochrome icon hues) instead of default Nerd Font + raw ANSI brightness.
- **Selection & focus** — a soft background plus a left accent bar, rather than a heavy full-row
  highlight.
- **Breathing room** — top/bottom margin, a little left padding, and a consistently-aligned
  right-hand git gutter.
- **Type treatment** — bold directories, tasteful dim/italic, maybe a thin colored left-edge on
  changed rows.

Verify by eye across real trees: deep nesting, git changes, compact chains, ignored/dimmed dirs,
and search highlighting.

## Design

Reframed as the **theme engine + the flagship `birch` theme** (ADR 0021). This task builds the
machinery and the one beautiful default; `025` adds the rest of the catalog.

**Types.** In `birch-tui`: a `Theme` struct plus `GuideStyle { None, Indent, Connectors }`,
`BadgeStyle`, `SelectionStyle { FullRow, SoftBarAccent }`, an `IconSet`, and a `Palette` (all the
colors currently const in `render.rs`). In `birch-core`: `ThemeId { Birch, Vscode, Jetbrains,
Xcode, Retro, Plain }` — a `Settings.theme` field (default `Birch`) and a `SettingKey::Theme`
variant. `Theme::for_id(ThemeId) -> Theme` lives in tui (only `Birch`/`Plain` fully realized here;
the others land in `025`).

**Wire-through.** `render.rs::draw` and `icons::icon_for` take `&Theme`; the app derives the active
theme from `Settings.theme` and threads it in. `ctl set theme` re-renders live (no restart). The
hardcoded consts become the `Birch` palette, re-tuned below.

**The `birch` flagship theme:**
- **Guides** `Indent` — a dim `│` in each ancestor indent column (the 2-col slots already there, so
  `hit_test` geometry is unchanged), brightened along the selected row's ancestry.
- **Selection** `SoftBarAccent` — a *soft* low-contrast background (lighter than today's
  `#2f3b54`) plus a left accent bar (`▏`) in the birch accent (green `#6f9152`, echoing the logo).
- **Palette** — curated and muted: near-neutral name fg, **bold directories**, one green accent,
  calmer git colors (mute the current VS Code hues a touch), ignored stays dim.
- **Badges** — unchanged shape (letter for files, `●` rollup for dirs) in the muted git colors.
- **Icons** — the current Nerd Font map, colors nudged toward the muted palette; folder glyph shown.
- **Breathing room** — one blank top row and a one-space left pad; git gutter stays right-aligned.
  If the left pad shifts the row origin, update `hit_test` in lockstep (render and hit-test share
  the geometry).

**Public surface** (user- and client-facing only; the internal types — `ThemeId`, `Theme` &
friends — are in the design above and ADR 0021):
- **Protocol:** a new socket-settable `theme` key (client-facing, additive; unknown to older
  clients, tolerated).
- **Behavior:** the default tree appearance changes for everyone (the new `birch` look).
- Theme *selection* (`--theme`, `birch ctl set theme`, config `theme`) lands in `025` / `031`.

**Tests:** theme resolution by id; span-level assertions that the `Birch` theme paints guides and
the accent-bar selection; `hit_test` still resolves rows/chevrons with the new left pad.
