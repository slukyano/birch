---
type: Sprint
title: Click model and detached open commands
status: Done
branch: sprint/010
tasks:
- 022-click-selects-first
- 023-detach-open-cmd
---

# Scope rationale

Second batch of live-use feedback, scope and both key decisions given by the
maintainer in chat: a click on an unselected row must select it, not immediately
open/toggle (double-click activates — maintainer's pick over second-click-on-selected);
and opening a preview must not blank the tree pane (birch suspends the TUI for every
custom `--open-cmd` as if it were a terminal editor — adapters need a detached mode,
`--open-detached` per the maintainer's naming call). Two tasks; crates + contrib + docs.

# Checklist

- [x] click-selects-first
- [x] detach-open-cmd

# Open questions

None — click semantics (double-click activates, chevron immediate, Enter unchanged)
and the flag name (`--open-detached`, matching `OpenMode::Detached`) were both decided
by the maintainer in chat.

# Sprint summary

- **click-selects-first** (high): single click selects only — tree, filter list,
  picker alike; a 450 ms path-keyed double-click is Enter's twin (open / toggle /
  pick); chevron clicks toggle immediately and disarm a pending double (ADR 0015,
  Accepted — reverses the design doc's VS Code-school rule after live pane use
  disproved its rationale). Detection is a pure `ClickTimer` in birch-tui.
- **detach-open-cmd** (high): `--open-detached` (requires `--open-cmd`) runs the
  template fire-and-forget — null stdio, no TUI handover — via the existing
  `OpenMode::Detached`; all three contrib adapters pass it, ending the tree-pane
  flash on every preview. Docs, recipes, and the adapter promise updated.
- **Maintainer fold-ins**: one folder glyph — the open-folder icon flip removed,
  the chevron alone carries expansion state (an open variant may return under
  visual styles). Alongside on mvp, outside the sprint: `mise run dev` build+subshell
  replaces the always-on in-repo PATH patch (`mise.dev.toml`, `MISE_ENV=dev`).
- **Independent review**: request-changes; applied — the picker filter list's
  chevron zone now counts as the name (a single click on a dir match could
  confirm-and-exit the picker), missing dirs report no chevron in hit_test,
  empty-space clicks disarm the pending double, README/recipe docs aligned;
  `resolve_click` extracted and covered by app-level tests.
- **Verification**: cargo test / clippy / fmt green; detached preview path
  live-verified in a separate debug cmux window (master | preview | tree, focus
  never moves, tree never suspends; single-preview reuse intact). The click model
  is unit/app-test covered; feel-testing by mouse is the maintainer's first-use.

# Session log

- Sprint created from live-use feedback; both design decisions made by the
  maintainer in chat before scoping.
- Implemented; review findings applied; folder-glyph fold-in; closed out
  and merged (mvp, then mvp → main on maintainer instruction).
