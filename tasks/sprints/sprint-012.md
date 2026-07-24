---
type: Sprint
title: Publishable repo & process docs
status: Done
branch: sprint/012
tasks:
- 036-add-license-file
- 037-remove-scratch-fixtures
- 039-split-workflow-doc
- 040-define-contribution-flow
- 041-restructure-tasks-bundle
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

- [x] 036-add-license-file
- [x] 037-remove-scratch-fixtures
- [x] 039-split-workflow-doc
- [x] 040-define-contribution-flow
- [x] 041-restructure-tasks-bundle

# Open questions

None raised during implementation. All decisions were resolved in the design phase: the
numbered-slug model (the number is part of the concept name) and renumber-all-now; scratch
fixtures removed going forward (no history rewrite); the MIT copyright year kept; and the
`CONTRIBUTING.md` location with a standard-PRs stance.

# Sprint summary

- **036-add-license-file** (minor): root `LICENSE` added — verbatim MIT, `Copyright (c) 2026
  Stanislav Lukyanov`. No manifest change (`license = "MIT"` already declared). Closes the
  "no license → all rights reserved" gap on the public repo.
- **037-remove-scratch-fixtures** (minor): `bar.md`, `bar2.md`, and `foo/` removed via a plain
  commit (no history rewrite); `.gitignore` now covers `.claude/`, `.cmux/`, `.readb`. The
  files remain reachable only in the root commit — an accepted trade to avoid a force-push.
- **039-split-workflow-doc** (major): `tasks/workflow.md` reworked — the publication-hygiene
  gate split into **hygiene** (no identifiable individuals except the copyright identity, no
  environment leakage, sourced other-project claims) and **voice** (impersonal and agentless;
  roles only in this document's governance statements); a scope-presentation format added under
  scoping; the new bundle layout documented; tooling-neutrality prose trimmed. The `AGENTS.md`
  hook was refreshed to match.
- **040-define-contribution-flow** (mid): root `CONTRIBUTING.md` as the functional-contribution
  guide — build/test, Conventional Commits, standard PRs welcome, scope-fence check, and the
  maintainer-vs-contributor note. The internal "no PRs" wording was reconciled (internal flow
  is PR-less; external contribution is standard GitHub).
- **041-restructure-tasks-bundle** (major): every task file renamed to `NNN-slug.md` (number in
  the concept name); 24 delivered tasks moved to `tasks/archive/`, 24 open kept in `tasks/`, 12
  sprint records moved to `tasks/sprints/`; every `blocked_by` and sprint `tasks:` reference
  rewritten to the numbered name; `index.md` links updated. Numbering is deterministic
  identity-order (delivered by sprint `001→011`, then open in index order), 3-digit and dense.

**Not done (deliberately deferred — each a named `Draft` task):** the distribution/polish
chunk — `038-add-ci-workflow`, `042-set-up-homebrew-tap`, `043-automate-releases`,
`044-document-installation`, `045-add-repo-demo`, `046-add-cargo-metadata`,
`047-decide-crates-io-publish`, `048-add-changelog`.

**Breaking changes:** the `tasks/` bundle layout changed — task files are now `NNN-slug.md`,
closed tasks live under `archive/`, sprints under `sprints/`. Internal references were all
updated; any external reference to an old task path breaks.

**Architectural decisions / ADRs:** none (workflow/doc changes, not birch architecture).

**Bugs found & fixed:** none in implementation. The independent review found one voice nit — a
second-person greeting in `CONTRIBUTING.md` versus the sprint's own new voice rule — fixed.

**Remaining limitations & highlights:**
- The `split-workflow-doc` trim was modest: the doc was already largely operational and the
  cross-repo meta already lives in the `sprint-workflow` skill (outside this repo), so little
  needed moving. The substantive work was the gate reformulation, the scope format, and the
  layout documentation.
- Sprint-body checklists in archived sprints still reference bare slugs (history, left as-is);
  only frontmatter references were renumbered.
- `docs/design.md` (the product spec) was untouched; this sprint was repo/process only.

# Session log

- Scoped and cut: five tasks across repo hygiene (LICENSE, scratch-fixture removal) and
  process docs (workflow split, contribution flow, tasks-bundle restructure). Branch
  `sprint/012` cut from `main`; `add-ci-workflow` and the distribution/polish tasks held for
  a later infrastructure sprint.
- Implemented all five; validation green; independent review clean (one voice nit fixed).
  Closed out — five tasks Done and moved to `tasks/archive/`, sprint Done.
