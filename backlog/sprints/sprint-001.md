---
type: Sprint
title: Foundation — name check and the core tree view
status: Done
branch: sprint/001
tasks:
- 001-verify-name-availability
- 002-build-core-tree-view
---

# Scope rationale

The two `high` tasks, and the only unblocked ones that matter: everything else in the
backlog is blocked on `build-core-tree-view`, and the name check must land before the name
hardens into the socket path scheme and docs. Small task count, but the core task carries
the two load-bearing architecture boundaries — it is the sprint.

# Checklist

- [x] verify-name-availability
- [x] build-core-tree-view

# Open questions

None outstanding; both design-phase questions (name conflict handling, source concurrency
model) were resolved by ADRs 0002 and 0004.

# Sprint summary

- **verify-name-availability** (minor): crates.io `birch` is taken by an unrelated tool;
  Homebrew formula/cask and all fallback crate names are free. Decision in ADR 0002: keep
  the name; Homebrew is the channel of record; `birch-tree` is the cargo-install fallback.
  Findings recorded in the task body.
- **build-core-tree-view** (major): phase 0.1 delivered — `birch-core` (path-keyed real
  tree mutated only by `TreeDelta`s, threaded Files source per ADR 0004, open-cmd
  templates, settings), `birch-tui` (pure flat-view view-model per ADR 0003, Nerd Font
  icon map, input mapping, renderer + hit-testing), `birch` binary (clap CLI with the 0.1
  defaults-table flags, unified event loop, terminal handover for terminal editors,
  detached platform opener). 37 unit tests; behavior verified end-to-end in a scripted
  PTY (draw, navigate, expand, quit).
- **Independent review**: one blocker (wheel scroll snapped back by selection-following)
  and four should-fixes (scroll clamp, input-thread race on editor handover, terminal
  state leak on partial setup failure, NodeId doc contract) found and fixed; Enter/Right
  semantics brought back in line with the design doc's keyboard table (expand only —
  the earlier toggle/step-into behavior was out of spec).
- **Known sharp edges**: the editor-handover input race is narrowed (120 ms park window +
  stale-input discard), not eliminated — keystrokes typed in that window can be lost.
  Re-expanding a dir does not drop entries deleted on disk; that staleness is the
  watcher's job (`add-live-updates`). Selection is unset until the first interaction.

# Session log

- Sprint created; scope approved.
- Designs approved (ADRs 0002–0004 accepted); design merge to the mainline.
- Implementation, gates, review fixes; sprint closed out.
