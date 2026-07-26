---
type: Task
title: Refine the tree's visual design — earn "beautiful"
description: Tree/indent guides, a curated palette, refined selection, breathing room. General render-layer polish, distinct from the choosable style presets (025).
status: Draft
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
