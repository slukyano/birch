---
type: Task
title: One picker — Enter always picks
status: Done
priority: high
---

Maintainer feedback: `--pick` vs `--pick-dir` is the wrong split. One `--pick` mode;
Enter confirms whatever is selected — file or dir; arrows browse.

## Design

- `--pick-dir` is removed (pre-release, no compatibility concern). `--pick` picks the
  selected row on Enter regardless of kind; `→`/`←` expand/collapse for browsing;
  Enter on the root row picks the root.
- Mouse in picker mode: clicking a file picks it; clicking a dir name or chevron
  toggles it (browsing); Enter remains the only dir-pick affordance — deliberate, so
  browsing clicks don't accidentally confirm.
- The `dirs_only` parameter disappears from the search API and Mode; the filter list
  already picks on Enter.
- Docs (README, integrations, design doc invocation section) update; the design doc's
  `--pick-dir` mention is replaced by the unified behavior. Future: a Finder-like
  filename filter (glob/regex) — `add-picker-filter` draft.
