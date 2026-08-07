---
type: Sprint
title: Branch state, and critical fixes
status: Designing
branch: sprint/018
tasks:
- 076-search-unusable-on-large-roots
- 077-quit-swallowed-during-terminal-handover
- 064-configurable-badge-placement
- 078-add-branch-diff-badges
---

# Scope rationale

Two defects and one feature that arrives with its own layout question.

The feature is `078`: badges for how a file differs from `main`, alongside the working-tree badges
birch has always had. It comes paired with `064` because it introduces a **second badge axis**, and
where badges live is currently hard-coded to two columns on the right. Designing them together
means the gutter is settled once — `064` establishes placement as a theme axis and a setting, and
`078` slots into that model rather than inventing a layout `064` would immediately generalise.

The defects lead. `076` is the sprint's only high-priority item and the one a user meets first: a
picker opened from a home-sized root cannot search at all, and says "no matches" while it means
"no index yet". `077` is small but a correctness hole — a quit asked for during a terminal handover
is answered and then ignored.

# In-scope task ledger

- **`076-search-unusable-on-large-roots`** — *bug, high, design-heavy.* `search::build_index` walks
  the whole root synchronously on one thread and yields nothing until the walk completes; `$HOME`
  is ~250 000 entries with no gitignore to prune it, so the index never arrives. Separately, the
  status line reports `(no matches)` whenever the index is merely absent — a statement about the
  corpus birch is in no position to make, and worth fixing independently of any speed work.
- **`077-quit-swallowed-during-terminal-handover`** — *minor (bug), medium.* `perform_open`'s
  stale-event drain calls `self.handle(...)` and discards the returned quit flag, so an
  `AppEvent::Shutdown` or a `ctl quit` arriving while a child owns the tty is consumed, answered
  `ok`, and ignored. Pre-existing; found by the sprint-017 independent review. The fix has to carry
  the quit out through the `NavEffect` path without losing the save-and-restore ordering a normal
  quit performs.
- **`064-configurable-badge-placement`** — *mid, medium.* Badge placement becomes a theme axis
  (`Right` / `Left` / `None`) beside the existing `BadgeStyle`, plus a setting across flag, config,
  and `ctl set`, with per-theme defaults grounded in the measured editors. `none` is not `--no-git`:
  status colours and rollups stay, only the column goes. Left placement shifts the row layout, so
  `hit_test` geometry must move in lockstep with the painted columns.
- **`078-add-branch-diff-badges`** — *mid, medium.* Badges marking how a file differs from `main`,
  as distinct from the working tree against `HEAD`. Open for design: whether it reuses the existing
  status vocabulary computed against the base, or a separate in/out mark; whether both directions
  or only outgoing; what "main" resolves to; how the badges refresh when branch state changes
  inside `.git`, which the watcher never sees; and how a second axis shares the gutter `064` defines.

# Ordering / dependencies

- **`076` first** — the critical defect, entirely independent of the rest.
- **`077` next** — small and independent, in the app-loop neighbourhood
  [ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md) has just reshaped.
- **`064` before `078`** — `064` settles what the gutter is, so `078`'s second axis slots into an
  established model instead of inventing one.

# Considered but out of scope

- **`035`** — high priority, but not agent-executable: it needs the maintainer at a live interactive
  herdr session.
- **`070`** — filter match counts. It competes for the same right-hand columns as `064` and `078`,
  which made it a tempting rider, but it is a filter feature rather than a git one and would blur
  the sprint. It inherits whatever gutter model this sprint leaves.
- **`071`, `028`, `029`, `034`** — the action surface. Unblocked now that `067` settled the click
  model, and the natural next sprint.
- **`072`, `074`** — still forbidden by the scope fence in `docs/design.md`; neither is designable
  until the fence is amended, which is a maintainer decision plus an ADR.
- **`026`, `051`, `053`, `073`** and the theme follow-ups (`055`, `056`, `058`, `061`, `065`, `066`)
  — off-theme for this sprint.
- **`030`, `032`, `033`** — the additional sources; "Later" in the design doc.

# Sprint-start action

Scope committed to `main`; branch `sprint/018` cut from it. Design phase opens with `076`, whose
reproduction and diagnosis are already recorded in the task.

# Checklist

- [ ] 076-search-unusable-on-large-roots
- [ ] 077-quit-swallowed-during-terminal-handover
- [ ] 064-configurable-badge-placement
- [ ] 078-add-branch-diff-badges

# Open questions

_(none yet)_

# Session log

- Scoped and cut: `076`, `077`, `064`, `078`. Branch `sprint/018` cut from `main`.
