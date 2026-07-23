---
type: Sprint
title: Find things — fuzzy search, picker mode, state persistence
status: Done
branch: sprint/003
tasks:
- add-fuzzy-filename-search
- add-picker-mode
- add-state-persistence
---

# Scope rationale

Phase 0.3 plus the two 0.4 items that don't touch the socket: picker mode is the search
engine's second render policy (designing them together keeps the engine honest), and state
persistence is the small independent task that makes the persistent-pane story survive
restarts. What remains for the final MVP sprint is exactly the integration story: socket +
birch-ctl + adapter/recipes.

# Checklist

- [x] add-fuzzy-filename-search
- [x] add-picker-mode
- [x] add-state-persistence

# Open questions

None outstanding.

# Sprint summary

- **add-fuzzy-filename-search** (major): ignore-walk index worker + nucleo matching
  (ADR 0009); jump-style UI (matches bold, everything else dim, arrows cycle, tree
  spatially stable); the `reveal()` primitive expands ancestors toward a match as loads
  arrive and doubles as the future socket verb. ⚠️ ADR 0008 resolved the design doc's
  internal conflict: `q` types into search, Ctrl-C quits — the keyboard table's `q` row is
  superseded by its own reserved-characters footnote.
- **add-picker-mode** (mid): `--pick`/`--pick-dir` on a stderr-backed terminal; with a
  query the pane filters to a flat best-first match list; Enter/click confirm eligible
  rows (chevron clicks still browse in `--pick-dir`); stdout carries exactly the picked
  path; cancel exits 1. Open-cmd, watcher, and persistence are inert in picker mode.
- **add-state-persistence** (minor): expansion/selection/scroll per root at
  `~/.cache/birch/<fnv1a64>.json` (root-relative real paths, version-gated, atomic
  writes); expansion saves on change, view state on quit; restores cascade as listings
  arrive and wait for the first git answer so dirs that became ignored stay collapsed —
  a race the PTY smoke test caught live.
- **Independent review**: no blockers; nine should-fixes applied — stale-reveal loops,
  reveal cancellation on abandoned searches, index-refresh yank, chain-interior reveal,
  a Duration-underflow panic in the index throttle, the persistence seed comparing
  absolute against relative paths, hidden picker status, the rebuild gate, and the
  missing app-layer test suite (eight tests added: reveal cascade/staleness, search
  transitions, cycling across index refreshes, restore gating, picker confirms).
- **Known sharp edges**: index rebuilds are whole-tree walks (throttled, off-thread) —
  incremental patching waits for measured need; match highlighting is row-level, not
  per-character; the picker's empty-query tree view allows browsing but confirming
  requires a query-selected or tree-selected eligible row.

# Session log

- Sprint created; scope approved.
- Designs approved (ADRs 0008–0009 accepted); design merge.
- Implementation, PTY-verified search/picker/persistence, review fixes,
  app-layer tests, sprint closed out.
