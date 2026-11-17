---
type: Sprint
title: Context highlights
status: Planned
branch: sprint/024
tasks:
- 100-add-context-highlights
---

# Scope rationale

One task, sprint-sized on its own. `100-add-context-highlights` turns the branch column of
[`078-add-branch-diff-badges`](../archive/078-add-branch-diff-badges.md) into a highlighted set
chosen at runtime: a diff against any ref, recent changes, changes since a commit, or a custom list
— set over the control socket by anything that drives birch, an agent in particular. It moves the
git worker, the watcher, a timer the time-based mode needs, the protocol and `birch ctl`, config and
flags, the render column, and the root annotation, which is more than shares a sprint comfortably
with anything else.

It runs after [`sprint-023`](sprint-023.md): the two touch disjoint code, and the adapters' fixes and
verification were scoped first.

The task's headline product decisions — the look, one active set, git-anchored recency, custom as a
mode with no `clear`, the `highlight` verb, and the fate of the `branch-badges` surface — were taken
before planning and are recorded in the task body. The design phase builds on them.

# In-scope task ledger

- **`100-add-context-highlights`** — *medium, design-heavy.* A highlighted set in the `078` column,
  one mode at a time (`branch [<ref>]`, `recent <duration>`, `since <commit>`, `custom <paths>`,
  `off`), set through a new `highlight` socket verb and `birch ctl highlight`, read back through
  `get-highlight`, chosen at launch by `highlight` / `--highlight`, never persisted at runtime.
  Likely one ADR.

# Considered but out of scope

- **`032-add-git-changes-source`** — a tree of only changed files. Source-shaped where this is a
  side-table; it could later reuse the same producers, but nothing here needs it.
- **`099-mark-symlinked-rows`** — wants a column in the same badge band. Independent of this task;
  pairing them would make one sprint argue about the band twice.

# Scope at start

Per the workflow, a `Planned` sprint's scope is re-checked against the backlog and re-presented for
approval when it starts.
