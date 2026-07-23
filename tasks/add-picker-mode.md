---
type: Task
title: Add picker mode
description: birch --pick / --pick-dir — search filters, Enter prints selection to stdout and exits.
status: Done
priority: medium
blocked_by:
- add-fuzzy-filename-search
---

Phase 0.4 (partial) of [the design doc](../docs/design.md): transient picker on the same UI —
search filters instead of jumps, Enter prints to stdout and exits, mutations disabled by
default. The adoption funnel: picker first, persistent pane once hooked.

## Design

`birch --pick [<dir>]` and `--pick-dir` (implies pick; restricts to dirs):

- **stdout stays clean for `$(birch --pick)`**: the TUI renders on **stderr**; the only
  bytes ever written to stdout are the selected path (absolute) plus newline. Confirming
  prints and exits 0; quitting without a selection (Ctrl-C / Esc on an empty query) exits
  1 printing nothing.
- **Filter render policy** (ADR 0009): with a live query the pane shows the match list as
  a dense flat list of relative paths, best match on top, selection moving over matches;
  with an empty query the normal tree shows (browsing is still available). `--pick-dir`
  builds the index from dirs only.
- **Enter = confirm** in the filtered list (prints the selected match). In the tree view:
  Enter on a file confirms in `--pick`; Enter on a dir confirms in `--pick-dir` and
  expands otherwise. Open-cmd is never invoked in picker mode; mouse click follows the
  same confirm rules.
- Git decoration stays (badges help disambiguation); watcher and persistence are off in
  picker mode (a transient picker must not overwrite the persistent pane's saved state).
- Implementation: `Mode::Tree | Mode::Pick { dirs_only }` in the app; the render module
  gains the flat-list draw + a query input line; `term` parameterizes over
  stdout/stderr.

**Tests**: filter-list assembly (ordering, dirs-only), confirm semantics per mode, and
the stdout contract (unit-level: the picker's confirm path returns the string to print
rather than printing deep in the stack).
