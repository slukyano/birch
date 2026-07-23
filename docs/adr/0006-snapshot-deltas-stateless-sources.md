---
type: ADR
title: Directory snapshots are a delta; sources stay stateless; the tree reconciles
status: Accepted
sprint: sprint-002
---

# Context

Live updates need removals: when a watched dir changes, entries may have vanished. The 0.1
delta language has merge-only `Added` plus single-path `Removed`/`Updated`. Either sources
diff old-vs-new listings themselves (requiring sources to hold tree state) or the tree
reconciles a full one-level listing against what it has.

# Decision

Add `TreeDelta::Snapshot { dir, entries }`: the authoritative one-level listing of a dir.
`Tree::apply` reconciles — inserts new names, updates kinds, removes subtrees of names no
longer present — while preserving surviving children's loaded/expanded state. The Files
source emits `Snapshot` for every `Expand` command; watcher-triggered refreshes reuse the
same command and delta. Sources remain stateless functions from commands to observations;
`Added`/`Removed`/`Updated` stay in the language for sources that genuinely observe
increments.

# Consequences

- A watcher event degrades to "re-scan one level of one dir" — cheap, self-healing (a
  missed event is corrected by the next scan), and immune to platform event-semantics
  quirks.
- The tree is the single owner of diffing; no source needs bookkeeping.
- Re-expanding a dir now also drops stale entries (fixing the 0.1 staleness noted in
  sprint-001's summary).
