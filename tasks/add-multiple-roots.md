---
type: Task
title: Support multiple roots
description: Several top-level roots in one instance, IDEA attach-project style.
status: Draft
priority: medium
---

Maintainer request: one birch instance showing several root dirs as sibling top-level
rows (IDEA's attached projects). Touches the tree model (one tree per root or a
virtual super-root), persistence keying, the socket verbs (`set-root` vs `add-root`),
watchers, and search index scoping. Needs a design phase.
