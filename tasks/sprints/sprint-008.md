---
type: Sprint
title: Third feedback batch — cmux adapter, chain splitting, {line} cleanup
status: Done
branch: sprint/008
tasks:
- 018-add-cmux-integration
- 019-split-chains-on-demand
- 020-drop-line-template
---

# Scope rationale

Third maintainer feedback batch. The maintainer now runs birch inside cmux, so the
design doc's named-but-never-built `birch-cmux` adapter becomes the live integration
testbed. The earlier chain-arrow report is clarified as a feature request: `→` on an
already-expanded chain should un-collapse the middle folders. And the `{line}`
template placeholder is premature future-proofing — drop it from the CLI now and move
the open-at-line contract into the content-search task, auditing on the way that every
not-yet-built part of the design doc is represented in the backlog.

# Checklist

- [x] add-cmux-integration
- [x] split-chains-on-demand
- [x] drop-line-template

# Open questions

None.

# Sprint summary

- **add-cmux-integration** (mid): `contrib/birch-cmux` — open/toggle/socket over the
  cmux CLI (`new-split`, `respawn-pane`, `close-surface`), one birch per workspace
  (socket keyed on a checksum of the workspace UUID — path-length and collision safe),
  find-by-title via `cmux tree`, best-effort split narrowing. Enter opens files as cmux
  **preview tabs** next to the launching terminal — deliberately not `$EDITOR`, since
  the main terminal in cmux usually runs an agent. Live-verified in the maintainer's
  session: spawn, focus-existing, toggle both ways, stale-socket respawn,
  reverse-reveal, preview-tab open.
- **split-chains-on-demand** (mid): `→` on an expanded compact chain splits it into
  member rows per ADR 0014; splitting is free (members are loaded by construction);
  any ancestor collapse prunes and re-fuses; split state is render-layer only, never
  persisted.
- **drop-line-template** (minor): `{}`-only open-cmd contract (path appended when
  absent); stale `{line}` templates now fail loudly at parse; the open-at-line contract
  moved to add-content-search; backlog audit confirmed every unbuilt design-doc element
  has a task (two compact-chain interactions added to add-file-operations).
- **Independent review**: approve-with-nits; applied — checksum socket key, strict
  `new-split` parse with loud failure, field-equality pane lookup, `{line}` template
  rejection, two ADR 0014 tests (splits survive deltas; ancestor collapse prunes), one
  drifted design.md line. Noted follow-up outside sprint scope: `Decor::home` is a dead
  field (`visible_rows` calls `env::home_dir` directly).
- **Incident**: the maintainer's cmux instance crashed during live testing — cmux's own
  markdown viewer (`MarkdownViewerAssets.loadAsset`) faulted in a process predating an
  on-disk app update; reproduced clean after relaunch. Not a birch or adapter defect.

# Session log

- Sprint created from maintainer feedback (cmux request + mid-turn
  clarifications on {line} and the chain-arrow feature).
- {line} dropped, chain splitting implemented (ADR 0014), birch-cmux
  shipped and live-verified in the maintainer's cmux session (one cmux-side crash
  diagnosed as an upstream stale-process bug), review fixes applied, closed out.
