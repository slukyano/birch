---
type: Task
title: Search match cycling jumps around instead of moving down the tree
description: ↑/↓ during a search step through matches in fuzzy-score order, so the selection teleports; they should walk matches in tree order.
status: Done
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

## Design

**Keep `search()` in score order.** Ranking is its job (ADR 0009/0013), so its contract and its
tests are untouched. The *app* holds the matches in tree order instead.

**A tree-order sort in core.** `birch-core/src/search.rs` gains:

```rust
/// Sorts matches into tree display order — the order `visible_rows` draws
/// them in, so stepping always travels visually down or up.
pub fn sort_tree_order(matches: &mut [Match])
```

The sort key of an entry is its `rel` path split into components, each carried as
`(is_file, name.to_lowercase())`; every non-final component is necessarily a directory, and the
final one takes the entry's own kind. Rust's lexicographic `Vec` ordering over that key reproduces
the display order exactly, because `visible_children` sorts each level by *directories first, then
case-insensitive name* — `false < true` puts dirs ahead of files at the level where they diverge,
and a shorter key (an ancestor) sorts before its descendants, which is where the tree draws it.
Decorate–sort–undecorate, so each key is built once per rematch.

**`SearchState.matches` is held in tree order**, and `current` indexes it directly. `cycle_match`
moves `current` by ±1 modulo `matches.len()` (wrap-around preserved) and reveals the entry.

**The selection anchors on position, not on score** (maintainer requirement, design phase). On
**every** rematch — the first typed character and each one after it — the selection moves to the
**first match at or after the current selection, in tree order**, and wraps to the first match when
none follows. If the currently selected row is itself a match, it stays. If there are no matches,
there is no selection at all (ADR 0023).

- The anchor is the current selection, so a narrowing keystroke that keeps the row a match never
  moves the view, and one that kills the match moves forward — never backward, never across the tree.
- Implemented as a binary search for the anchor's tree-order key over the sorted matches, so the cost
  stays logarithmic in the match count.
- Score order now has **no consumer**: the flat picker list is deleted by `062`, and the initial
  selection is positional. `search()` keeps returning best-first anyway — the ranking is meaningful
  and cheap, and a future "jump to best match" would want it.
- The `keep_position` parameter of `rematch` disappears. The anchor rule already does the right
  thing after an index rebuild: it re-finds the selection, or the next match after where it was.

**Stepping still expands.** `cycle_match` keeps calling `reveal`, so a match inside a collapsed
directory is revealed rather than skipped (the task's preferred reading). Because the order is
positional, successive reveals are monotone: expanding to reach match *k* only inserts rows above
match *k+1*, never reorders them.

**Public surface.** None. No new flags, config keys, protocol fields, environment variables, or
on-disk paths; `search()`'s signature and ordering contract are unchanged, and `sort_tree_order` is a
new function on an internal-consumer crate (`birch-core` is not a published library).

**Tests.**

- Core: `sort_tree_order` over a scrambled match set yields directories before files at the same
  level, case-insensitive name order, and an ancestor directory before its descendants.
- App: `↑`/`↓` visit paths in tree order, wrapping at both ends.
- App: the selection anchors forward — from a row above the first match it lands on the first match;
  from between two matches it lands on the following one; from a row after the last match it wraps
  to the first.
- App: a selected row that still matches the narrowed query does not move.
- App: no matches leaves no selection.
