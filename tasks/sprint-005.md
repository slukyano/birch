---
type: Sprint
title: MVP feedback — quit keys, root row, VISUAL
status: Done
branch: sprint/005
tasks:
- fix-quit-keys
- show-root-row
- prefer-visual-editor
---

# Scope rationale

First-use feedback from the maintainer on the merged MVP, batched as one small sprint:
Ctrl-C-only quitting feels wrong (Esc should back out), the tree should show its root as
a row, and the open default should honor `$VISUAL`. Scope and direction came from the
maintainer directly, so the designs are recorded per task and implementation starts
immediately.

# Checklist

- [x] fix-quit-keys
- [x] show-root-row
- [x] prefer-visual-editor

# Open questions

None.

# Sprint summary

- **fix-quit-keys** (mid): Esc backs out one layer — search first, then the app
  (ADR 0012); Ctrl-C stays unconditional. Design doc keyboard table updated (also
  retiring the stale `q` quit row per ADR 0008).
- **show-root-row** (mid): the root renders as row zero, children nest one level deeper;
  never chains, visibility-exempt, collapsible. In `--pick-dir`, Enter on the root row
  confirms the root (documented decision). Recorded in the design doc's Appearance
  section.
- **prefer-visual-editor** (minor): default open resolves `$VISUAL`, then `$EDITOR`,
  then the platform opener; blank values fall through; testable resolution function.
- **Independent review**: no blockers; fixed — reveal stalling against a collapsed root
  (and not focusing a root target), a duplicate root scan at every launch, the spec gap
  for the root row, and three missing tests (Esc quit branch, collapsed-root reveal,
  Enter-on-root pick).
- All three changes PTY-verified live (Esc idle-quit and search-then-quit, `▾ smoke2`
  root row, VISUAL beating EDITOR).

# Session log

- Sprint created from maintainer feedback; designs approved with scope.
- Implemented, PTY-verified, review fixes applied, closed out.
