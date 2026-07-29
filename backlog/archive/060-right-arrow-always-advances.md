---
type: Task
title: → should always advance — never a silent no-op
description: Right-arrow does nothing on files and on already-expanded plain dirs; it should descend into a folder or move to the next row, doing nothing only on the tree's last file.
status: Done
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

## Design

**The rule.** `→` resolves in three cases, first match wins:

| selection | `→` does |
|---|---|
| a collapsed directory (not deleted-but-tracked) | **expands it**; the selection stays put |
| an expanded compact chain | **splits it** into member rows (ADR 0014, unchanged) |
| anything else | **moves to the next live row** |

It is a no-op only when there is no next live row — the last row of the tree (and, under an active
narrowing, when nothing live follows).

**Answers to the task's two open questions.**

1. **A collapsed folder is entered in two presses, not one** (maintainer decision during design):
   the first `→` expands, the second moves into it. The task's original table asked for "move to the
   first element, expanding first when collapsed"; one press cannot honour that faithfully, because
   an unloaded directory's children arrive asynchronously (`NavEffect::RequestExpand` → a delta
   round-trip), so it would need a pending-descend state machine whose visible behaviour differs
   between loaded and unloaded directories. Expanding *is* a visible action, so "never a silent
   no-op" holds.
2. **Chain splitting keeps `→`** (ADR 0014 stands). `→` reveals structure when there is structure to
   reveal — expand a collapsed directory, split an expanded chain — and advances otherwise. No key
   has to be found for splitting, which matters because printable characters are reserved for search.

**Does `→` subsume `↓`?** Nearly, and deliberately not entirely. Because directories sort before
files, "next sibling file, else the parent's next sibling" *is* the next visible row, so on files
`→` and `↓` coincide. They differ exactly where it is useful: on a **collapsed directory** `↓` steps
over it while `→` opens it, and on an **expanded chain** `→` splits. Under an active search `↑`/`↓`
step *matches* (`063`) while `→` walks live rows, so the two keys separate again.

**Liveness.** "Next row" means next **live** row under ADR 0023 — `→` skips dimmed rows exactly as
`↓` does. This is the same `FlatView` skipping logic `062` introduces; no separate implementation.

**Implementation.** `FlatView::on_right` (`crates/birch-tui/src/flat_view.rs:470`) gains the final
branch: where it currently falls through to `NavEffect::None`, it calls the same
"advance to the next live row" helper `move_by` uses. Roughly ten lines, no new state.

**Docs.** `docs/design.md`'s keyboard table currently describes `→` as "expand"; it becomes
"expand / split / advance" with the table above. The README's key list follows.

**Public surface.** None — no flags, config keys, protocol fields, environment variables, on-disk
paths, or public APIs. A keyboard behaviour change, documented in `docs/design.md` and the README.

**Tests.**

- A file with a following sibling advances to it.
- The last file of a folder advances to the parent's next sibling.
- The last row of the tree does not move.
- A collapsed directory expands and the selection stays; a second `→` then enters it.
- An expanded compact chain splits (ADR 0014 regression), and does not advance.
- An expanded plain directory advances into its first child.
- Under an active narrowing, `→` skips dimmed rows.
