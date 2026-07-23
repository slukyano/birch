---
type: Task
title: Add the Git Changes source
description: A third source listing changed files, reusing the source-as-delta-stream interface.
status: Draft
priority: low
blocked_by:
- add-git-status
- add-content-search
---

Sequenced "Later" in [the design doc](../docs/design.md): a Git Changes source — the tree
pane showing only changed files, VS Code SCM-view style. Falls out of the sources
architecture once Files and Content Search have validated the interface; no new UI surface.
