---
type: Task
title: Show how much a filter actually matches
status: Draft
priority: low
blocked_by:
- 027-add-picker-filter
description: A filter keeps every folder navigable, so a browse can wander through folders holding nothing pickable; surface the counts so the tree tells you where to go.
---

Maintainer observation while using `--filter`: because directories are never dimmed — deliberately,
so the tree stays walkable — a filtered browse can wander through folders that hold nothing
selectable at all. Finder ultimately behaves the same way, so this is a refinement rather than a
correction of `027`.

## Direction

Tell the user where the matches are, instead of making them look:

- a **count per directory** (matching files in its subtree), rendered like the git rollup;
- and/or a **total** on the status line: `filter: *.md (37 files)`.

## The hard part

The count is only knowable for **loaded** directories — the tree loads lazily, and an unread
listing may hold anything. So a count is either partial (and must not look authoritative) or needs
a background walk of its own. The search index already walks the whole root and is filter-aware
since `027`; deriving counts from it is the obvious cheap route, at the cost of being as fresh as
the index rather than as fresh as the tree.

Also weigh: a count column competes with the badge gutter (`064`) and the scrollbar (`068`) for the
right-hand edge.
