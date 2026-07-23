---
type: Task
title: Show the root as the first tree row
description: The root renders as row zero with its children nested, VS Code-style.
status: Done
priority: medium
---

Maintainer feedback on the MVP: the root lived only in the status bar; the tree should
show it as a node.

## Design

`flat_view::visible_rows` emits the root as an explicit first row (depth 0; children
shift to depth 1). The root row behaves like any dir row — chevron, click, collapse
(collapsing leaves just the row itself) — with two exceptions: it never joins a compact
chain (a single-child root must not fuse into `root/sub`), and it is never a synthetic
or ignored row. The first `sync` selects it by default, `←` from a top-level child jumps
to it, and `get-path` on it already prints `.`.

In `--pick-dir` the root row is confirm-eligible: a bare Enter picks the root itself —
an explicit "this dir" answer, pinned by a test.

Startup always expands the root, so a collapsed-root quit restarts expanded; the
persistence format is unchanged (the root was already filtered from the expansion set).
The status bar keeps the full root path — the row shows the directory name.
