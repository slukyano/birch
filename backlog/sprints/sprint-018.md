---
type: Sprint
title: Branch state, and critical fixes
status: Done
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

- [x] 076-search-unusable-on-large-roots
- [x] 077-quit-swallowed-during-terminal-handover
- [x] 064-configurable-badge-placement
- [x] 078-add-branch-diff-badges

# Open questions

_(none yet)_

# Sprint summary

Search became usable on a real machine, and the tree learned to say what a branch would merge.

**`076` was measured before it was designed, and the measurement changed the design.** The task
proposed parallelising the index walk. Timing it said otherwise: parallelising is 1.66× once entries
are actually constructed, which leaves the same symptom. What mattered was that nothing was
searchable until a walk of 825,782 entries finished, and that the index was ten times larger than
the data needed — 263 MB where a parent-plus-name representation measures 25 MB. fzf was the
reference point, and instructive: it is not frugal either, and has no faster walk. It *streams*.
So does birch now.

**`078` went through four shapes before settling.** Letters against the base, then letters with a
modifier, then a single mark — each simplification came from the maintainer, and each removed
something the user would have had to learn. The glyph outlasted the design: `⎇` was withdrawn after
its coverage was measured, because it renders on macOS only by borrowing from a proportional system
font — the same substitution that sheared the indent guides in `059`.

**The independent review found a crash this sprint introduced.** A pane one to three columns wide
killed birch, and the guard test written for exactly that case missed it because its rows painted
nothing in the gutter. The lesson is in the test now: it paints.

# Task ledger

| Task | Weight | What changed |
|---|---|---|
| [`076-search-unusable-on-large-roots`](../archive/076-search-unusable-on-large-roots.md) | **major** | Search works from a home-sized root: first results in ~300 ms rather than never, and "no matches" is only said once there is a complete corpus to say it about. Planned as a faster walk; measurement redirected it to a smaller index published as it grows. |
| [`077-quit-swallowed-during-terminal-handover`](../archive/077-quit-swallowed-during-terminal-handover.md) | minor | Quitting while a file is open in an editor now quits. Delivered as designed. |
| [`064-configurable-badge-placement`](../archive/064-configurable-badge-placement.md) | mid | Status badges sit right, left, or nowhere, per theme and overridable. Delivered as designed. |
| [`078-add-branch-diff-badges`](../archive/078-add-branch-diff-badges.md) | mid | A column marking what this branch would merge. Filed as in/out badges with status letters; settled on one mark in one column, outgoing only, after four rounds of maintainer simplification. |

# Public-surface delta

| Surface | Addition |
|---|---|
| CLI | `--badges <right\|left\|none\|auto>`, `--no-branch-badges` (and hidden `--branch-badges`) |
| Config | `badges`, `branch-badges` |
| Socket | `SettingKey::Badges`, `SettingKey::BranchBadges` — additive only |
| `birch ctl` | `set badges <right\|left\|none\|auto>`, `set branch-badges <on\|off\|toggle>` |

`076` and `077` add none. No breaking changes. One vocabulary for `badges` across all three
surfaces, checked deliberately after the review found three.

# Architectural decisions

- **[ADR 0026](../../docs/adr/0026-the-index-is-a-tree-published-progressively.md)** — the search
  index is a tree of parent-plus-name entries, published in immutable chunks while the walk runs,
  with absent / building / complete states so `(no matches)` is a claim birch may make.

# Bugs found & fixed

By the **independent review** — four majors, three verified by execution:

- **A pane 1–3 columns wide panicked and killed birch.** The badge band was placed past the buffer
  once the gutter had been clamped. The guard test missed it because its rows painted nothing;
  it now paints a status and a mark and sweeps both placements and both axis states, and was
  verified to panic against the previous commit.
- **A rebuild threw away every published chunk** and restarted at zero, so under filesystem churn
  the index never reached complete — defeating the one claim ADR 0026 exists to make. The walk now
  always finishes.
- **Left-hand placement put the branch mark outermost**, breaking both of `078`'s invariants.
- **An arriving chunk moved the cursor** whenever the selection was not itself a match — legal
  under ADR 0023 — up to four times a second for the length of the walk.

Nine minors: a non-UTF-8 name silently reparenting its whole subtree; an index from a previous root
adopted and called complete; three vocabularies for `badges`; `resolve_base` able to resolve a *tag*
named `origin/main`; the "fetched" age reading a file that does not exist once refs are packed; the
repo root always carrying the mark; a file the branch *deletes* never marked; the branch diff
re-running on every status refresh, now cached against the two commits it compares; and the missing
CHANGELOG entries.

Found while implementing: re-matching on every published chunk **doubled the walk** while a query
was live (10.1 s against 5.2 s idle), fixed with a 250 ms cadence. And a nested `Option` made clap
panic on every `--badges` invocation — missed first time because the check used `--help`, which
exits before values are read.

# Remaining limitations & highlights

- **A file the branch deletes has no row to mark** unless git still reports it: it is absent from
  disk, so the tree has nothing to draw. The column answers "what would I merge, of what exists".
- **The branch comparison is only as fresh as the last fetch**, and birch does not fetch. The root
  row says how stale it is.
- **A path-mode query** (one containing `/`) reconstructs each candidate's path and allocates while
  doing so; the default name-mode reads the slab and allocates only for hits.
- **The index walk is unchanged in duration** — streaming makes it stop mattering, not stop taking
  seconds. With a query live throughout it is ~8 s on a home directory against ~5 s idle.
- **`⎇` is unusable** as a branch mark and the reasons are recorded in `078`: no common terminal
  font has it.

# Verification

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — green;
  **230 tests** (66 + 70 + 94).
- Hands-on: search from `$HOME` (203 matches at 0.6 s, 1,176 at completion, peak RSS 316 MB →
  43.7 MB); `ctl quit` during a real editor handover, with the fix reverted to confirm the bug
  reproduces; all three badge placements and both icon modes on screen; the branch column checked
  against `git diff origin/main...HEAD`; every `--badges` value run rather than `--help`-ed.
- Glyph coverage and advance widths measured from the `cmap` and `hmtx` tables of Menlo, SF Mono,
  JetBrains Mono, Courier New, Andale Mono and the JetBrains Nerd Font variants.

# Session log

- Scoped and cut: `076`, `077`, `064`, `078`. Branch `sprint/018` cut from `main`.
- Design phase: `076-search-unusable-on-large-roots` measured — 9.4 s warm, 19.1 s cold, 263 MB —
  which disproved the task's own suspicion that parallelising the walk was the fix (1.66× once
  entries are constructed) and showed the index is ten times larger than the data needs. Became
  **ADR 0026**: a tree-shaped index published in chunks while it builds, with absent/building/
  complete states so `(no matches)` becomes a claim birch may make.
  `078-add-branch-diff-badges` went through several shapes at the maintainer's direction and settled
  on **one** `⎇` column meaning "in this branch's change set", with the local badge outermost and
  immovable; the remote-branch comparison was dropped as a near-subset of the base one.
- Design approved: ADR 0026 `Proposed → Accepted`; the four tasks `Draft → Designed`; the sprint
  `Designing → Implementing`. Design merge to `main`.
- Implementation: `076` (ADR 0026 — a tree-shaped index published in chunks), `077` (the quit
  propagated out of the handover drain), `064` (badge placement as a theme axis and a setting),
  `078` (the branch column). Independent review: four majors — a narrow-pane panic, a rebuild that
  discarded published chunks, the mark outermost under left placement, and a cursor that moved as
  chunks arrived — plus nine minors, all fixed. The branch glyph was withdrawn and replaced after
  its font coverage was measured. Closed out; gates green.
