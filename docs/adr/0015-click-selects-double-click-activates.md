---
type: ADR
title: Click selects, double-click activates
status: Accepted
sprint: sprint-010
---

# Context

The design doc prescribed the VS Code school: "single-click file → open, single-click
dir → toggle", on the rationale that selection-without-action has no purpose in a
sidebar. Live pane-host use disproved the rationale: when birch runs in a host pane
(cmux, tmux), a click meant only to focus the pane — or merely to point at a row —
is delivered to birch as a normal click, and birch cannot distinguish it from an
intentional one. First click on an unfocused tree opened files and toggled dirs by
accident. Selection-without-action turns out to be exactly what a harmless first
click needs to be.

# Decision

- **Single click selects only** — file or dir, tree or filter list, picker or not.
- **Double-click activates**: a second click on the same row within 450 ms is
  Enter's twin — open file, toggle dir, confirm filter-list entry, pick in picker
  mode. The Midnight Commander school over the second-click-on-selected model: a
  re-click on the selection minutes later must do nothing, or the original accident
  returns through the side door.
- **Chevron clicks keep toggling immediately**, without moving selection and without
  double-click semantics — each press is its own toggle. A chevron click disarms a
  pending double (chevron-then-name in quick succession is a select, not an open).
- Double-click detection is keyed on the row **path**, not the visual index, so live
  tree updates between the two clicks cannot activate the wrong row.

# Consequences

- Opening a file by mouse costs two clicks. Enter, the chevron, and all keyboard
  behavior are unchanged.
- Picker mode gets strictly safer: exploratory clicks never confirm; only
  double-click picks.
- The input layer stays pure position mapping; the double-click window is
  render-layer state (a small timer in birch-tui), testable without a terminal.
- The design doc's Mouse, Open, and Picker sections are amended accordingly.
