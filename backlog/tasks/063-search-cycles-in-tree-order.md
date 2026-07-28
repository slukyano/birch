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

## Design

**Keep `search()` in score order.** Ranking is its job (ADR 0009/0013) and the initial selection
depends on it — the fix is not to re-sort the result, but to give *stepping* its own order.

**A tree-order permutation in core.** `birch-core/src/search.rs` gains:

```rust
/// Indices of `matches` in tree display order — the order `visible_rows`
/// draws them in, so stepping always travels visually down or up.
pub fn tree_order(matches: &[Match]) -> Vec<usize>
```

The sort key of an entry is its `rel` path split into components, each carried as
`(is_file, name.to_lowercase())`; every non-final component is necessarily a directory, and the
final one takes the entry's own kind. Rust's lexicographic `Vec` ordering over that key reproduces
the display order exactly, because `visible_children` sorts each level by *directories first, then
case-insensitive name* — `false < true` puts dirs ahead of files at the level where they diverge,
and a shorter key (an ancestor) sorts before its descendants, which is where the tree draws it.
Decorate–sort–undecorate, so each key is built once per rematch.

**Stepping walks the permutation.** `SearchState` gains `order: Vec<usize>` and `current` becomes an
index **into `order`** (not into `matches`):

- `rematch` recomputes `order` alongside `matches`; the initial `current` is the position of the
  best-scoring match — `order.iter().position(|&i| i == 0)` — so the *selection* is still relevance-driven
  while *stepping* is positional.
- `cycle_match` moves `current` by ±1 modulo `order.len()` (wrap-around preserved) and reveals
  `matches[order[current]]`.
- `rematch(keep_position = true)` — the index-rebuild path — re-finds the **current match's path** in
  the new `order` instead of keeping a raw position, which is meaningless once the match set changed;
  it falls back to the best match when that path is gone.

**Stepping still expands.** `cycle_match` keeps calling `reveal`, so a match inside a collapsed
directory is revealed rather than skipped (the task's preferred reading). Because `order` is
positional, successive reveals are monotone: expanding to reach match *k* only inserts rows above
match *k+1*, never reorders them.

**Public surface.** None. No new flags, config keys, protocol fields, environment variables, or
on-disk paths; `search()`'s signature and ordering contract are unchanged, and `tree_order` is a new
function on an internal-consumer crate (`birch-core` is not a published library).

**Tests.**

- Core: `tree_order` over a scrambled match set yields directories before files at the same level,
  case-insensitive name order, and an ancestor directory before its descendants.
- App: with an index whose best-scoring match sits in the middle of the tree, the initial selection
  is the best match, and `↑`/`↓` then visit paths in tree order, wrapping at both ends.
