---
type: ADR
title: Printable characters win — q types into search; Ctrl-C quits
status: Accepted
sprint: sprint-003
---

# Context

The design doc's keyboard table lists `q` / `Ctrl-C` as Quit, and the same section states
the stronger rule: "printable characters are permanently reserved for search — no letter
hotkeys, ever." Once any-printable-char starts a fuzzy search, the two are in direct
conflict: with `q` as quit, no filename containing a leading `q` (`quux.rs`) is reachable
by typing, and "typing searches" grows an asterisk. A quit key that only works when search
is inactive would make `q` behave differently depending on invisible state.

# Decision

The reserved-characters principle wins: from the moment fuzzy search exists, `q` is a
printable character like any other — it types into the search query. Quit is `Ctrl-C`
(and later the context menu / `birch-ctl quit`). This supersedes the keyboard table's `q`
row; the table's own footnote states the principle this decision enforces.

# Consequences

- No modal behavior, no letter hotkeys, no exceptions — the invariant is simple to state
  and to keep.
- Users habituated to `q`-quits TUIs will hit a search for "q" instead; the status bar
  showing the live query makes the state visible, and `Esc` clears it. `--help` documents
  Ctrl-C.
