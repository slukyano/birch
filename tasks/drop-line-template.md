---
type: Task
title: Drop {line} from the open-cmd template
description: Remove the unused {line} placeholder from the CLI; the open-at-line contract ships with content search instead.
status: Done
priority: medium
---

Maintainer decision reversing the v0.1 future-proofing: advertising `{line}` while
nothing can ever supply a line is out of place. The template contract grows the
placeholder when content search — the feature that produces line numbers — actually
lands.

## Design

- `OpenCmd::build` loses its `line` parameter and the `{line}` substitution/dropping
  logic; templates are `{}`-only. `--open-cmd` help, design doc, README, and
  integration examples say only that.
- [add-content-search](add-content-search.md) now owns the open-at-line contract: it
  reintroduces `{line}` (with the drop-args-when-absent behavior) as part of phase
  0.6, and says so explicitly.
- Backlog audit — every not-yet-built part of the design doc must be a task. Sweep
  result: all sections map to existing drafts; two mentions added to
  [add-file-operations](add-file-operations.md) that were only in the design doc
  (F2 on a chain edits the full `a/b/c` fragment; mouse segment-clicks scope the
  context menu to individual chain members).
