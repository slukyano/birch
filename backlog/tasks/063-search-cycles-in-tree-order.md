---
type: Task
title: Search match cycling jumps around instead of moving down the tree
description: ↑/↓ during a search step through matches in fuzzy-score order, so the selection teleports; they should walk matches in tree order.
status: Draft
priority: high
---

Maintainer report: while searching, going forward/backward jumps around like crazy.

## Cause

`search()` ranks results and returns them **sorted by score**
(`crates/birch-core/src/search.rs:128` — `sort_by_key(Reverse(score))`), and `cycle_match`
(`crates/birch/src/app.rs:805`) simply steps `current` through that vector and reveals the entry.
So `↑`/`↓` follow *relevance* order, which bears no relation to what is on screen: consecutive
presses can throw the selection from the top of the tree to the bottom and back.

## Expected

`↑`/`↓` during a search move to the **next / previous match in tree order** — the same top-to-bottom
order the rows are drawn in — so the selection always travels visually downward or upward.
Relevance ordering stays useful for *ranking* (which match is selected first when the query
changes, and any future "best match" jump), but not for stepping.

## Notes

- Keep the initial selection on the best-scoring match when the query updates; only the *stepping*
  order changes to positional.
- Matches include entries in collapsed directories, which have no row on screen — decide whether
  stepping expands to reveal them (today `cycle_match` calls `reveal`, which expands) or skips to
  the next visible one. Expanding is probably right; it should still move monotonically.
- Wrap-around at the ends should be preserved.
- Cover with a test: given a scored-out-of-order match set, cycling visits paths in tree order.

Related: `062` (picker/tree search divergence).
