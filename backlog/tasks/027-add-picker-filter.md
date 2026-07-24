---
type: Task
title: Add a picker file filter
description: Finder-like filename filter for picker mode - glob or regex pattern.
status: Draft
priority: low
blocked_by:
- 016-unify-picker
---

Maintainer request: picker mode gains a filename filter (macOS Finder-style) — a glob
or regex restricting which entries are pickable/listed (e.g. `*.md`). Likely a flag
(`--filter '*.md'`) applied to the index and tree rows in picker mode; interacts with
the fuzzy query (filter restricts the corpus, the query ranks it).
