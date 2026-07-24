---
type: Task
title: Deduplicate and route the documentation set
description: Give each topic a single home across README, AGENTS, CONTRIBUTING, workflow.md, and docs/; add a docs/ index; date the logs.
status: Draft
priority: high
---

Deduplicate overlapping guidance and route each topic to a single home across the documentation
set — `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `backlog/workflow.md`, `docs/design.md`,
`docs/integrations.md`, `backlog/index.md` — with clear progressive-disclosure pointers between
them. Add a `docs/` bundle index (and log), and date the change logs.

## Target roles (single home per topic)

- **README.md** — the entry point for *users*: what birch is, how to use it, how to install, and
  how to build from source. Points to `CONTRIBUTING.md` for development.
- **CONTRIBUTING.md** — conventions and the issue/PR flow (drop the scope-fence paragraph — no
  need to warn contributors about the small scope); a shallow **repository map** (not full
  depth); the **tech-docs approach** (what lives in `docs/`, and when to update it); the
  build/run/test commands; and a short **development-workflow note** (the maintainer develops via
  the agentic workflow in `backlog/workflow.md`; others need not follow it). Everything except
  the workflow note and the issue/PR flow is useful to humans *and* agents — see the open
  question on separating that shared core.
- **AGENTS.md** — the entry point for *all* agents (the maintainer's and external). Points to
  `CONTRIBUTING.md` and `README.md` with one-line descriptions for progressive disclosure; keeps
  the hard rules; does not duplicate what the linked docs already say; points to
  `backlog/workflow.md` **conditionally** — only when the agent is instructed to follow the
  maintainer's workflow.
- **backlog/workflow.md** — the maintainer's agentic sprint workflow (the conditional deep-dive).
- **docs/design.md** — the product and architecture spec (binding scope fence).
- **docs/integrations.md** — the host-adapter integration guide.
- **docs/index.md** (new) — a listing for the `docs/` bundle.
- **backlog/index.md** — the backlog listing.

## Also

- **Add `docs/index.md`** (and likely `docs/log.md`) so `docs/` is a navigable bundle like
  `backlog/` and `docs/adr/`.
- **Date the change logs.** `backlog/log.md` (and the new `docs/log.md`) should carry dates. This
  revisits the publication de-dating decision for logs specifically: task/sprint frontmatter
  stays date-free, but a *log* is a chronology and reads wrong without dates. Update
  `backlog/workflow.md`'s log guidance to match.

## Open questions (for design)

- **Separate CONTRIBUTING's shared core?** Everything in `CONTRIBUTING.md` except the issue/PR
  flow and the workflow note is equally useful to humans and agents. Split that shared core into
  its own doc (referenced by both `CONTRIBUTING.md` and `AGENTS.md`), or keep it in
  `CONTRIBUTING.md` and have `AGENTS.md` point at the relevant sections?
- **How much of a bundle is `docs/`?** Just `index.md` + `log.md`, or a full OKF bundle with
  per-doc concepts/frontmatter?
- **Repository-map depth and home** — how shallow, and confirm `CONTRIBUTING.md` is its home.
- **Dating format** — date-prefixed log entries vs a dated-heading style; reconcile with the
  publication-hygiene rule and the de-dating done earlier.
- **Progressive-disclosure wiring** — the exact set of cross-links and their one-line descriptions
  between README ↔ CONTRIBUTING ↔ AGENTS ↔ workflow.md ↔ docs/.

## Coordination

Overlaps the deferred README tasks: `044-document-installation` (the install section) and
`045-add-repo-demo` (a demo). This task fixes README's *role and routing*; those fill specific
sections. Sequence them so the routing lands first.

Design-heavy and touches binding docs (`docs/design.md` scope fence, the `AGENTS.md` hard rules,
`backlog/workflow.md`): belongs in a sprint's design phase, not an ad-hoc edit.
