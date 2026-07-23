---
type: ADR
title: Esc backs out one layer at a time — and quits at the top level
status: Accepted
sprint: sprint-005
---

# Context

ADR 0008 gave every printable character — including `q` — to search, leaving Ctrl-C as
the only quit. First real use showed that feels wrong: a TUI needs a discoverable,
low-ceremony way out. The design doc's keyboard table assigned Esc only "clear search /
close menu" and predates the `q` resolution.

# Decision

Esc dismisses the innermost active surface, one per press: an open menu (when it exists,
0.5), then an active search (restoring the pre-search view), and with nothing left to
dismiss, the app itself. The picker already behaved this way (Esc on an empty query
cancels the pick); the persistent pane now matches. `Ctrl-C` remains the unconditional
quit at any layer. The design doc's keyboard table is updated to state both.

# Consequences

- One mental model — "Esc backs out" — covers menus, search, and the app, matching the
  broader TUI convention (fzf, lazygit).
- An accidental double-Esc can close the pane; state persistence makes the relaunch
  land exactly where the pane was, which keeps the cost near zero.
