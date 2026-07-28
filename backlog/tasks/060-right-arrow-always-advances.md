---
type: Task
title: → should always advance — never a silent no-op
description: Right-arrow does nothing on files and on already-expanded plain dirs; it should descend into a folder or move to the next row, doing nothing only on the tree's last file.
status: Draft
priority: medium
---

Maintainer report: pressing `→` should **always** do something. The only position where it may do
nothing is the very last file of the tree.

## Requested behaviour

| selection | `→` does |
|---|---|
| a folder | move to the **first element** of that folder (expanding it first when collapsed) |
| a file with a following sibling | move to the **next sibling file** |
| the last file of a folder | move to the **next folder** — the parent's next sibling |
| the last file of the tree | nothing |

## Current behaviour

`FlatView::on_right` (`crates/birch-tui/src/flat_view.rs`):

- collapsed dir → expands it, selection stays put (no descent);
- expanded **compact chain** → splits the chain into its member rows (ADR 0014);
- expanded plain dir → **nothing**;
- any file → **nothing**.

So two of the four cases above are silent no-ops today, and expanding never moves the selection.

## Design questions for the maintainer

1. **Does `→` subsume `↓`?** Because directories sort before files, "next sibling file, else the
   parent's next sibling" is exactly *the next visible row*. Together with "descend into a
   folder", the requested rule reduces to: *expand if collapsed, then move to the next visible
   row*. That makes `→` equivalent to `↓` everywhere except on a collapsed folder. Intended?
2. **What happens to chain splitting?** `→` on an expanded compact chain currently splits it
   (ADR 0014). Does splitting still take priority, does it split *and* advance, or does it move
   to another key? An ADR amendment or superseding ADR may be needed.
3. **`docs/design.md`'s keyboard table** describes `→` as "expand"; it needs updating with
   whatever is decided.

Note `←` is already asymmetric in the requested direction — it collapses an expanded dir,
otherwise jumps to the parent — so "always does something" is the established behaviour on that
side.
