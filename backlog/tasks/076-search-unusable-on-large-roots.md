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
