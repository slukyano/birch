---
type: Task
title: Click selects, double-click activates
description: Single click moves selection only; double-click opens/toggles; chevron still toggles immediately. Reverses the design doc's single-click-activates rule.
status: Done
priority: high
---

Live cmux use showed the flaw in "single-click file → open" (design doc, VS Code
school): a click meant to focus the tree pane — or just to point at a row — opens the
file or toggles the dir immediately. The host focuses the pane *and* delivers the
click; birch cannot tell a focusing click from an intentional one, so the first click
must be harmless. Maintainer decision: click selects; double-click activates
(time-bounded, Midnight Commander school); a late re-click on the selection does
nothing. Requires a design-doc amendment and an ADR.

## Design

- **Semantics** (maintainer's pick, ADR 0015): single click on a row **selects only**
  — file or dir, tree or filter list, picker or not. Double-click (same row within
  the window) is Enter's twin: open file / toggle dir / confirm filter-list entry /
  pick in picker mode. Chevron clicks keep toggling immediately without moving
  selection — the chevron is a pointing-only affordance with no destructive meaning,
  and each press is its own toggle (no double-click semantics on chevrons). Enter and
  all keyboard behavior unchanged.
- **Detection is app-side state, input stays pure**: terminals emit only `Down`
  events, so double-click is inferred. `input.rs` keeps mapping position-only
  `Click`s; a small pure `ClickTimer` in `birch-tui` holds the last click
  (path, `Instant`) and answers "is this a double?" — keyed on the row **path**, not
  the index, so live-update reshuffles between clicks can't misfire. Window: 450 ms
  (`DOUBLE_CLICK_WINDOW`). After a double it disarms (triple-click starts a fresh
  cycle); a chevron click disarms it too (chevron-then-name fast is select, not open).
- **Wiring**: the app's `Click` arm asks the timer; chevron or double → the existing
  `activate` path (picker semantics fall out unchanged: double-click picks a file,
  browses a dir — strictly better for "exploratory clicks never confirm by
  accident"); single → a new select-only `FlatView` entry point. `on_click` loses its
  built-in activate for name clicks.
- **Docs**: design doc Mouse section rewritten (single-click select, double-click
  activate, rationale: the host focuses the pane *and* delivers the click, so the
  first click must be harmless); Open and Picker sections updated; ADR 0015 records
  the reversal of the "VS Code school" rule.
- **Tests**: `ClickTimer` unit tests (double within window; late re-click; row
  change resets; disarm cases) and a `FlatView` test that a single name-click moves
  selection without toggling or opening.
