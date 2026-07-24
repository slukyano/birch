---
type: Sprint
title: Live decorated tree — git status, live updates, compact folders
status: Done
branch: sprint/002
tasks:
- 003-add-git-status
- 004-add-live-updates
- 005-add-compact-folders
---

# Scope rationale

Phase 0.2 exactly: the three tasks that turn the static tree into the product's visual
identity — git badges with ancestor propagation, watcher-driven live updates, and compact
folder chains. They interlock (chains split/fuse on live updates; ignored dirs constrain
the watcher; badges update from the same event flow), so designing and shipping them
together avoids seams.

# Checklist

- [x] add-git-status
- [x] add-live-updates
- [x] add-compact-folders

# Open questions

The design doc's heavy-churn question is answered for the MVP: path-keyed selection plus
follow-only-on-move scrolling make churn structurally unable to move the selection, and
the debouncer's max-latency cap bounds refresh cadence. Revisit only on real friction.

# Sprint summary

- **add-git-status** (major): GitState side-table from `git status --porcelain=v2 -z
  --ignored=matching -uall` on a worker thread (ADR 0005). Letter badges in a right-hand
  column, status-colored names, dir rollup dots via path-component propagation (works for
  collapsed/unloaded dirs), deleted-but-tracked files as synthetic rows, ignored dimming
  with `--hide-ignored`, `--no-git`. ⚠️ Transformed from the design doc's gix listing to
  the git CLI — recorded and reasoned in ADR 0005.
- **add-live-updates** (major): notify-based watcher, one non-recursive watch per
  displayed dir (root + expanded + chain members + .git and refs dirs), debounced
  dirty-dir batches (80 ms quiet / 400 ms max latency), dirty dir → one-level re-scan via
  the Snapshot reconcile delta (ADR 0006); every batch also refreshes git state. Landed
  watches confirm with a synthetic re-scan to close the expand-to-watch gap.
- **add-compact-folders** (mid): flatten-time chain compaction (ADR 0007) with
  dim-separator labels, tail-keyed interaction, split/fuse for free on re-flatten.
  ⚠️ The peek rule grew during implementation: every unloaded non-ignored real dir in the
  viewport gets a one-level load (ADR 0007 amendment) — required for collapsed top-level
  chains. Chains exclude symlinked dirs and never cross the ignore boundary.
- **Independent review**: two blockers found and fixed — the ignored-flags combination
  (`traditional` + `-uall` never emits dir records; switched to `matching` + real-repo
  integration test) and unbounded symlink-cycle peek loops (symlinks excluded from
  chains/peeks). Plus: watcher command latency (merged inbox), dir/file kind transitions
  dropping stale subtrees, peek/git-state ordering, .git refs watching, RD rename
  handling, max-latency debounce cap.
- **Known sharp edges**: ignored-dir detection depends on git ≥ 2.x porcelain v2
  behavior; a modified submodule shows a badge on its parent rollup but not on its own
  dir row; `--show-noise` with an expanded huge `.git` generates watcher traffic on
  every git operation.

# Session log

- Sprint created; scope approved.
- Designs approved (ADRs 0005–0007 accepted); design merge to the mainline.
- Implementation, PTY-verified live updates/badges/chains, review fixes
  (two blockers), sprint closed out.
