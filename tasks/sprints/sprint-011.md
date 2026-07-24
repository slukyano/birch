---
type: Sprint
title: cmux Dock integration
status: Done
branch: main
tasks:
- 024-adopt-cmux-dock-integration
---

# Scope rationale

Extended live-use design work with the maintainer moved the cmux integration off the
workspace-split adapter and onto cmux's right-sidebar **Dock** (a beta feature). The
split adapter had irreducible open flicker (cmux splits are born at 50% and resized)
and per-workspace socket complexity; the Dock hosts a persistent control declaratively
and is the natural home for a file tree. This sprint rewrites `contrib/birch-cmux`
around the Dock, records the decision as an ADR, and documents the setup. contrib +
docs only — no birch/Rust changes (it uses existing `--socket`, `--open-cmd`,
`--open-detached`, and `birch-ctl set-root`/`reveal`).

Executed autonomously at the maintainer's direction ("do it all, land it on main") after
the full design was validated live in throwaway cmux windows. Per that direction the
project also collapses onto `main`: the `mvp` integration branch and stale sprint
branches are retired.

# Checklist

- [x] adopt-cmux-dock-integration

# Sprint summary

- **adopt-cmux-dock-integration**: `contrib/birch-cmux` rewritten around the cmux Dock
  (ADR 0016). Three verbs — `dock-run` (dock.json entrypoint: resolve the window, start
  a scoped follow watcher, exec birch rooted at the window's selected workspace),
  `preview` (open a file as a tab in the window's main pane — birch's `--open-cmd`), and
  `dock-socket` (address this window's birch for reverse-reveal). One birch per window,
  socket keyed on `cksum(window-uuid)`; the follow watcher re-roots the tree on
  `workspace.selected` and dies with the window (pty SIGHUP) or cmux (event-stream EOF).
  The old split/toggle/preview-pane machinery is gone; the cmux adapter is dock-only
  (no non-dock fallback). Docs: integrations guide rewritten with a cmux-Dock section
  (enable flag, `dock.json`, verbs, reverse-reveal), design-doc integration note.
- **Verification**: no Rust changes, so `cargo test`/`clippy`/`fmt` are unaffected and
  green; the real validation was a full live regression in throwaway cmux windows —
  per-window socket binding, preview-as-tab (no split, no bootstrap terminal), the
  follow watcher tracking workspace switches, and clean teardown (birch + watcher both
  die on window close). Independent review of the diff + publication-hygiene pass.
- **Beta dependency (recorded risk)**: the integration rides the cmux Dock beta
  (`rightSidebar.beta.dock.enabled`) and its observed behavior — `dock.json` schema,
  per-window seeding, `workspace.selected`/`current-window`, no window id in the dock
  shell's env. If cmux changes these, the adapter needs updating; birch itself does not.

# Session log

- Sprint created from an extended live design session; scope, both preview
  recipe and the follow-watcher decisions are the maintainer's. Implemented and
  live-validated the adapter, wrote the ADR and docs, ran gates, and consolidated the
  project onto `main` (retiring `mvp` and stale sprint branches) per maintainer direction.
