---
type: Task
title: Add the content search source
description: Ctrl-F swaps the pane's source to files-with-matches, built on the ripgrep crates.
status: Draft
priority: low
blocked_by:
- 002-build-core-tree-view
---

Phase 0.6 of [the design doc](../docs/design.md): content search as a second source — same
tree widget, nodes are files-with-matches expandable to match lines, live debounced
incremental search on the ripgrep crates (`grep-searcher`, `grep-regex`, `ignore`), Enter
on a match line opens at that line. Validates the source interface.

This task owns the open-at-line template contract: it adds `{line}` to `--open-cmd`
(removed from the CLI in sprint-008 as premature), with args containing `{line}` dropped
when no line applies so `nvim +{line} {}` degrades to `nvim <path>` in the Files source.
