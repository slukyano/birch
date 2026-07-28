---
type: Task
title: Highlight the active folder's indent guide
description: Opt-in theme axis — dim every indent guide except the current folder's column, which brightens. Works for indent lines and classic connectors alike.
status: Draft
priority: medium
blocked_by:
- 054-refine-tree-visual-design
---

Maintainer request: indent guides should be **slightly more dimmed** everywhere except the column
belonging to the **current folder**, which renders **slightly brighter** — so the eye can follow the
active branch down the tree. This is what VS Code's active indent guide and IntelliJ's New UI
selected-path highlight do (see [editor ground truth](../../docs/research/editor-trees.md)); the
theme engine deliberately left it open when the flagship landed.

## The current folder

Resolved from the selection:

| selection | current folder |
|---|---|
| a file | its parent directory |
| a directory, expanded | **that directory** |
| a directory, collapsed | its parent directory |

## Behaviour

For the current folder at depth `a`, the guide column `a` brightens — but only on rows inside that
folder's subtree; sibling subtrees at the same depth keep the dimmed colour. Every other guide
column dims.

When the current folder is the root (depth 0) nothing brightens: the root spine column is
deliberately not drawn.

Applies to both `GuideStyle::Indent` (`│`) and `GuideStyle::Connectors` (`├─`/`└─`/`│`) — the
active column's glyph takes the bright colour whichever style is in use.

## Opt-in per theme

Off by default; a theme opts in. Two candidate surfaces for the design phase:

- one field — `Palette.guide_active: Option<Color>`: `None` keeps today's uniform guides; `Some`
  turns the feature on, with the theme's existing `guide` serving as the dimmed colour;
- two fields — an explicit dim/bright pair, so a theme can opt in without re-tuning `guide`.

## Implementation notes

`indent_spans` currently sees only the row it paints (`render.rs:234`), so the active column has to
be threaded in. Because `visible_rows` is a flat ordered list, the current folder's subtree is a
**contiguous run of rows** — resolve the folder once per frame from the selection, compute its row
range and depth, and pass `(active_depth, range)` down; a row inside the range brightens the guide
at column `active_depth`. Cheaper and less error-prone than a per-row path-prefix test.

Interaction to settle: the flagship fades guides with depth (`Theme::guide_fade`). Does the active
colour replace the faded value outright, or brighten relative to it? Depth-fade plus active-path
highlighting must not fight each other.

Verify by eye at several depths, on a sibling subtree (which must stay dim), with compact chains,
and under both guide styles.
