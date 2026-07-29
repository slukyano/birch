---
type: Task
title: Add file operations
description: Rename/delete/new inline ops with git-aware delete - the rest of the 0.5 bundle split off to 071 (context menu) and 028 (copy paths).
status: Draft
priority: medium
blocked_by:
- 002-build-core-tree-view
---

Phase 0.5 of [the design doc](../../docs/design.md): exactly four ops (rename, delete-to-trash,
new file, new dir) as inline row editing, with git-aware delete confirmation. The other two thirds
of the 0.5 bundle are their own tasks — `071-add-context-menu` (the menu and hover highlight) and
`028-add-copy-paths` (copy name / relative path / absolute path over the OSC 52-first chain) — so
this task is the op layer alone: the mutations, and the watcher-event ownership that keeps the tree
from flickering or jumping mid-edit.

Compact-chain specifics from the design doc land here too: F2 on a chain inline-edits
the full `a/b/c` fragment (rename-with-path reused), and mouse segment-clicks target
individual chain members so a menu or op scopes to the clicked dir.

Design-phase open question (from the design doc): trash on exotic filesystems / NFS —
fallback behavior when trash is unavailable.
