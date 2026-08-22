---
type: ADR
title: The inline editor is birch's only mode, and printables follow the focused field
status: Proposed
sprint: sprint-021
---

# Context

birch has no modes. The identity says so by contrast — *"Not broot — no verbs, no modal behavior.
Dead simple: arrows navigate, typing searches, Enter opens"* — and
[ADR 0008](0008-q-types-into-search.md) makes it mechanical: printable characters are search input,
never a hotkey, so there is no letter that means a command and therefore no state in which letters
mean different things.

File operations break that, and cannot avoid breaking it. `docs/design.md` chose **inline row
editing** for rename and create, over a bottom prompt bar, on the grounds that *"prompt bars grow
verbs"*. Either way a name has to be typed, and while it is being typed the letters must reach the
name rather than the search. There is no arrangement of the feature in which that is not true.

So the question is not whether to have this state — it is whether admitting it costs birch the rule
that keeps it out of broot's shape.

# Decision

**ADR 0008 is about there being no letter *commands*, not about search specifically. Printable
characters go to the focused text field; the tree's field is the search box, and an inline edit
temporarily focuses a different one.**

Read that way the rule is unchanged and the editor is not an exception to it. No letter acquires a
meaning; typing goes where a cursor is visibly sitting, as it always did.

1. **The inline editor is the only mode birch has**, and this decision is not a precedent for a
   second. What makes it admissible is a property no vim mode has: it is **self-evident, transient,
   and singly-exited**. A text cursor sits in a row on screen, `Enter` commits, `Esc` cancels, and
   there is no way to be in the state without seeing that you are.
2. **`Esc` gains a layer**: edit → menu → search → quit ([ADR 0012](0012-esc-backs-out.md)). The
   menu closes when it invokes an action, so a menu and an edit are never open together.
3. **The editor is a paint-time overlay, not a tree mutation.** A rename draws over the row; a create
   draws a **phantom row** under its target directory. `Tree` is untouched until `Enter`, so the
   real-tree/render boundary holds and an abandoned edit — including one abandoned by birch dying —
   leaves nothing behind.
4. **The overlay is anchored to a real path** (rename) or to a parent and a position (create), so
   live updates that reshuffle rows carry it along. The text lives in the editor, never in the tree,
   so nothing arriving from the watcher can disturb what is being typed.
5. **Confirmations use `Enter` and `Esc`, never `y`/`n`.** A letter key answering a prompt is exactly
   the letter command ADR 0008 forbids, and the two keys already mean commit and back out.

# Consequences

- **birch acquires a modal state, and the identity's "no modal behavior" now needs the qualifier
  above** to stay true. `docs/design.md` says so explicitly rather than leaving the contradiction for
  a reader to find.
- **A second mode has to clear the same bar**: visible, transient, one exit. Anything reached by a
  letter, or that can be entered without noticing, is refused by this decision rather than debated
  afresh.
- **`y`/`n` prompts are permanently unavailable**, which constrains every future confirmation to the
  status line plus `Enter`/`Esc`. That is a real limit and an intended one.
- **The editor needs no watcher suppression.** Because it never touches the tree, deltas continue to
  flow during an edit, and the snapshot reconciliation that already exists
  ([ADR 0006](0006-snapshot-deltas-stateless-sources.md)) needs no special case.
- **Search is unavailable during an edit**, since printables are going elsewhere. `Esc` first, then
  type. This is the one place the two text fields compete, and the layered back-out is the answer.

# Alternatives considered

- **A bottom prompt bar.** Rejected by `docs/design.md` before this task: a prompt bar is a place to
  put verbs, and the first one invites the second. It also separates the name being typed from the
  row it belongs to, which is the thing inline editing gets right.
- **Keep printables on search and edit with something else** — arrow-driven character pickers,
  external `$EDITOR`. Rejected as absurd for typing a file name; the second also hands the terminal
  away for a rename.
- **Declare the editor "not a mode" and move on.** Rejected as dishonest. It is a mode; what makes
  it acceptable is a stated bar that a future proposal must also clear.
