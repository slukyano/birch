---
type: Task
title: Search is unusable on a large root
description: On a root the size of a home directory the index never arrives, and the status line reports "no matches" — a claim it cannot make while no index exists.
status: Draft
priority: high
---

Maintainer report, from live use: search did not work under `--pick` from `~`.

## Reproduced

Typing `birch` in `birch --pick` started from `$HOME` shows `> birch (no matches)` and keeps
showing it — still no matches after 60 s, on a machine where `~/workspaces/personal/birch` plainly
exists.

Two things the reproduction settles:

- **Not `--pick`.** Tree mode from `$HOME` behaves identically (`search: birch (no matches)` after
  30 s). The picker is only where it is noticed first, because search *is* the picker's primary
  function.
- **Not search itself.** The same binary in a repo-sized root answers instantly: typing `render` in
  `birch --pick` from the birch checkout gives `> render (1/4)`.

## What is known

`search::build_index` walks the whole root **synchronously and on one thread**
(`ignore::WalkBuilder::build()` is the serial iterator; the crate's `build_parallel` is unused) and
returns only when the walk is complete. Nothing exists to search against until then.

A home directory is the case that breaks it:

- `find ~` had not finished after 60 s, having reported 249 735 entries;
- `~/Library` alone holds ~180 000 and `~/.cache` ~148 000;
- **nothing prunes them.** `$HOME` is not a git repository, so gitignore rules remove nothing, and
  neither directory is "noise" by birch's definition. Hidden entries are shown by default, so the
  dot-directories are walked too.

So the index is minutes of work at best, and every keystroke before it lands is answered from an
absent index.

## The dishonest part

While no index exists the status line says **`(no matches)`** — a statement about the corpus that
birch is in no position to make. Whatever is decided about speed, a query against a missing index
must not report an empty result; the two states are different and the user cannot tell them apart.
This half is independent of the walk's cost and worth fixing on its own.

## Direction

Weigh, and decide which combination is the design:

- **Say what is true** — `indexing…`, and a match count only once there is an index to count.
- **Walk in parallel** (`WalkBuilder::build_parallel`), the cheapest large constant-factor win.
- **Publish partial results** as the walk proceeds, so a big root becomes progressively searchable
  instead of binary.
- **Bound the walk** by depth or entry count, and say so rather than silently truncating — a
  silent cap makes "no matches" wrong in a new way.
- **Search what the tree has already loaded** first, deepening in the background: the tree itself
  stays responsive from `$HOME` today precisely because it is lazy.

Pruning `~/Library` and friends by name is the tempting shortcut and should be resisted: it is a
guess about the user's filesystem, and the out-of-scope reasoning that keeps birch from
auto-expanding ignored dirs applies to inventing an ignore list too.

## Also observed

Launching from `$HOME` puts a permission error in the status line — `cannot read
/Users/…/.Trash`, which is not readable on macOS. A root will routinely contain entries the
process cannot read, so an unreadable child is normal rather than newsworthy; whether it deserves
the status line at all is part of this task or its own.

## Design

### Measured

`build_index` was timed against `$HOME` (827,724 entries) with the filesystem cache warm, and
against a parallel walk doing the same work — building entries, not merely counting them:

| | time | notes |
|---|---|---|
| serial `build_index`, cold-ish cache | **19.1 s** | the shipped implementation |
| serial `build_index`, warm cache | **9.4 s** | best case, repeated run |
| parallel walk, entries built | **5.7 s** | `build_parallel`, one core per thread |
| parallel walk, counting only | 1.2 s | not the real workload — construction dominates |
| index memory | **316 MB peak RSS** | 263 MB logical, over 825,744 entries |

### What the numbers rule out

**Parallelising is not the fix.** The walk-only figure (1.2 s, 15× faster) is misleading: once the
entries are actually constructed the speedup collapses to **1.66×**, because allocation and
collection dominate, not directory traversal. 9.4 s becomes 5.7 s — still unusable, and worse from
cold.

**Memory is large, but mostly self-inflicted.** Per entry: 80 B of inline struct, `rel` averaging
109 B, `abs` 126 B, `name` 20 B. Home-directory paths are long — `Library/Caches` and `node_modules`
chains — but each path is also stored roughly **2.3 times**: `abs` is only root + `/` + `rel`, and
`name` is the tail of `rel`, for which `name_offset` already exists. That redundancy is **115 MB,
44 % of the total**, and removing it (deriving `abs`, slicing `name`) also shrinks the struct from
80 B to ~32 B — about **263 MB → ~110 MB**, mechanically, with no change to the model.

So the honest conclusion is narrower than "eager indexing is impossible": the *time* has no
constant-factor rescue — 9.4 s warm, 5.7 s parallel, worse cold — while the *memory* has an obvious
one that should be taken regardless of which model is chosen. What remains open is whether ~110 MB
and several seconds are acceptable for a home-sized root, or whether the corpus itself must shrink.

### The honesty half — settled, and independent

While no index exists, a query must not answer `(no matches)`. That is a claim about the corpus
birch cannot make, and it is why the bug reads as "search is broken" rather than "search is not
ready". The status line distinguishes three states — no index yet, an index that is partial, and a
complete index — and a count is only shown for the last. This half is worth landing whatever is
decided about the model, and it is what makes any bounded model honest rather than silently wrong.

### The model — open

Three candidates, differing in what the corpus *is*:

- **Bounded** — walk until a cap (entries or time), stop, and say the index is partial. Bounded
  memory and predictable cost; results are incomplete, so every "no matches" needs the qualifier.
- **Progressive** — publish the index in chunks as the walk proceeds, so search is usable within a
  second and improves while the user types. Combined with the parallel walk it makes the first
  results nearly immediate. Memory still climbs unless it is also capped, and a growing corpus
  interacts with the forward anchor and the "dimmed is inert" rules of
  [ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md): matches may appear behind
  the selection after it has settled.
- **Tree-first** — search only what the tree has already loaded, deepening in the background. Cheap
  and instantly correct, but a fresh picker has almost nothing loaded, which is the case that
  matters most.

These compose: progressive *with* a cap is bounded memory plus immediate usefulness, at the cost of
being the most involved of the three.
