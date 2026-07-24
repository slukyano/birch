---
type: Sprint
title: Second feedback batch — visuals, picker, CLI truth
status: Done
branch: sprint/007
tasks:
- 015-polish-tree-visuals
- 016-unify-picker
- 017-cli-truthfulness
---

# Scope rationale

Second maintainer feedback batch after real use: a LICENSE icon gap, search highlighting
too subtle (the reference is IDEA's boxed match fragments), the root row missing its
path annotation, the two-flag picker being worse than one Enter-always-picks mode, and
two CLI truth issues (--open-cmd help text, socket-by-default without an off switch).
Four future ideas land as Draft tasks. Scope and direction from the maintainer directly.

# Checklist

- [x] polish-tree-visuals
- [x] unify-picker
- [x] cli-truthfulness

# Open questions

- The "pressing → on an already expanded chain un-collapses the folders" report does not
  reproduce (verified stepwise, with and without git). Awaiting maintainer detail —
  possibly a request for VS Code-style per-segment chain navigation rather than a bug.

# Sprint summary

- **polish-tree-visuals** (mid): LICENSE-family icons; IDEA-style match boxes (amber
  background, dark text); the root row carries a dim `~`-abbreviated path annotation and
  the idle bottom line stays clear for messages.
- **unify-picker** (mid): one `--pick` — Enter picks the selection, file or dir; mouse
  clicks pick files and browse dirs (filter-list dir clicks are a deliberate no-op);
  `--pick-dir` and the search API's `dirs_only` removed; docs updated everywhere live.
- **cli-truthfulness** (minor): `--open-cmd` help states the VISUAL/EDITOR default and
  the reserved `{line}`; `--no-socket` opts out of binding and conflicts with an
  explicit `--socket`.
- **Independent review**: no blockers; eight findings applied — the unimplemented
  bottom-line half of the annotation design, two stale spec sentences, the filter-list
  dir-click inconsistency, the `--no-socket`-vs-`--socket` silent override, per-frame
  home lookups, missing `abbreviate_home` tests, and the undocumented opt-out.
- Four future drafts seeded: visual styles, multiple roots, picker filter, copy paths.

# Session log

- Sprint created from maintainer feedback; designs approved with scope.
- Implemented, tmux-verified, review fixes applied, closed out. The chain
  `→` report stayed open (not reproducible; question back to the maintainer).
