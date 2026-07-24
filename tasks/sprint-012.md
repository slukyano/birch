---
type: Sprint
title: Publishable repo & process docs
status: Implementing
branch: sprint/012
tasks:
- add-license-file
- remove-scratch-fixtures
- split-workflow-doc
- define-contribution-flow
- restructure-tasks-bundle
---

# Scope rationale

Doc and repo-structure work to make the repository publishable — everything a reader or
cloner encounters that is not a product feature. Two strands:

- **Repo hygiene** — an MIT `LICENSE` matching the Cargo manifest, and removal of the
  committed scratch fixtures (`bar.md`, `bar2.md`, `foo/`) with a tightened `.gitignore`.
- **Process docs** — trim `workflow.md` to its operational core with the meta moved to the
  `sprint-workflow` skill; an external contribution flow distinct from the agentic workflow;
  and a tasks-bundle restructure (numbered slugs, `tasks/archive/`, `tasks/sprints/`).

Build, CI, and packaging are deliberately held for a separate infrastructure sprint.
`split-workflow-doc` and `restructure-tasks-bundle` are bundled because both edit
`workflow.md` and the second moves the very files the first describes; designing and
implementing them together avoids two passes and a re-migration.

# Checklist

- [ ] add-license-file
- [ ] remove-scratch-fixtures
- [ ] split-workflow-doc
- [ ] define-contribution-flow
- [ ] restructure-tasks-bundle

# Open questions

None yet — the interactive design phase opens next. Known design inputs are already captured
in the task bodies: the hygiene-gate reformulation and the scope-presentation format
(`split-workflow-doc`); the numbering / archive / sprints-subdir migration implications, which
ripple through concept names, `blocked_by`, and `index.md` links (`restructure-tasks-bundle`);
and the purge-from-root-commit vs. remove-going-forward call for the scratch fixtures
(`remove-scratch-fixtures`).

# Session log

- Scoped and cut: five tasks across repo hygiene (LICENSE, scratch-fixture removal) and
  process docs (workflow split, contribution flow, tasks-bundle restructure). Branch
  `sprint/012` cut from `main`; `add-ci-workflow` and the distribution/polish tasks held for
  a later infrastructure sprint.
