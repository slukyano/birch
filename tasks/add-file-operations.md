---
type: Task
title: Add file operations, context menu, and copy paths
description: Rename/delete/new inline ops, right-click context menu, hover highlight, copy name/paths over OSC 52.
status: Draft
priority: medium
blocked_by:
- build-core-tree-view
---

Phase 0.5 of [the design doc](../docs/design.md): exactly four ops (rename, delete-to-trash,
new file, new dir) as inline row editing; git-aware delete confirmation; the right-click
context menu as the primary action surface; hover highlight; copy name / relative path /
absolute path with the OSC 52-first clipboard fallback chain.

Compact-chain specifics from the design doc land here too: F2 on a chain inline-edits
the full `a/b/c` fragment (rename-with-path reused), and mouse segment-clicks target
individual chain members so the context menu scopes to the clicked dir.

Design-phase open question (from the design doc): trash on exotic filesystems / NFS —
fallback behavior when trash is unavailable.
