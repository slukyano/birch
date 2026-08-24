---
type: Task
title: Mark symlinked rows
description: The tree knows which rows are links and paints them identically to real files and dirs; a marker in the badge band would say so.
status: Draft
priority: low
---

A symlink is indistinguishable from the thing it points at. `crates/birch-core/src/files_source.rs:58-61`
classifies every entry as `NodeKind::SymlinkDir` or `NodeKind::SymlinkFile`, `Row` carries that kind
into the render layer, and both consumers then throw the distinction away: `NodeKind::is_dir()` folds
`SymlinkDir` into `Dir` (`crates/birch-core/src/tree.rs:21-24`), so `icons.rs:94` and `icons.rs:120`
hand a linked directory the folder icon and a linked file its extension icon. Nothing in the row says
"link".

The proposal is the IDE convention: a small arrow, in the badge band beside the git status. The data
is already there, and this is a paint-time transform over a kind the real tree already holds — the
render-layer half of `docs/design.md`'s second boundary, with no core work implied.

## Scope

The panel's half holds: IDEA's project tree overlays a link arrow on the icon of a symlinked node.
**Verify the VS Code claim before designing** — the explorer's decoration column is where git status
letters and problem markers sit, and whether a symlink earns a decoration there (rather than only a
tooltip) is the premise this task was filed on. Nothing turns on which of the two it is; the design
answers a different question if only one of them does it.

The pane's half is where the cost sits, and it is the whole design. See below.

## What a design has to answer

- **Which column, and what it costs in width.** The badge band already carries two axes: the
  working-tree status outermost and the branch mark inward of it (`render.rs:688-729`), sized
  `BADGE_WIDTH` + `BRANCH_BADGE_WIDTH`. A third axis widens the gutter on **every** row, in a pane
  whose narrowness is half the scope fence. The alternatives each cost something else: replacing the
  type icon loses the file type (a terminal cell takes no overlay, which is how IDEA affords it), and
  a suffix after the name moves with the name and collides with compaction.
- **Collision with the status it shares a column with.** A modified symlink has two things to say in
  one cell. Either the band grows or one of them wins, and which one wins is the decision.
- **Where the marker goes when the band is not drawn.** `BadgePlacement::None` is a theme's default
  (`theme.rs:293`) and `--badges none` is a user's choice; a marker that lives only in the gutter
  disappears with it. Whether that is correct or whether the marker needs a second home is a
  question the branch mark never had to answer, because a branch mark without git badges is
  incoherent and a symlink without them is not.
- **Dangling links.** `files_source.rs:60` renders a broken symlink as a plain file — the target
  metadata call failed, and the failure is discarded. Both IDEs distinguish a dead link from a live
  one, and doing the same here means keeping that error rather than dropping it.
- **The glyph, in two fonts.** The precedent is `BRANCH_MARK_ICON` / `BRANCH_MARK_PLAIN`
  (`render.rs:24-31`): a Nerd Font glyph plus a fallback that is present in the common terminal
  fonts and advances exactly one cell. A marker that needs font substitution breaks the column it
  sits in.
- **The colour.** Hues are theme-owned, so a marker colour is a new `palette` entry across every
  built-in theme, not a constant.

## Related

- `078-add-branch-diff-badges` (archived) — the last task to add an axis to this band, and the
  closest precedent for what it costs.
- `064-configurable-badge-placement` (archived) — why the band has a placement setting at all.
- Out of scope unless a design argues otherwise: showing the **target** path. A tree row has nowhere
  to put one, and birch has no tooltip.
