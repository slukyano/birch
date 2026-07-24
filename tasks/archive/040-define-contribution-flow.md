---
type: Task
title: Define the external contribution flow
description: A minimal, honest contribution framing distinct from the agentic development workflow.
status: Done
priority: high
---

The agentic sprint workflow is how the project is *produced*; it is not a contribution
guide. Define a small, honest framing for outside humans — issues welcome, development is
agent-driven, this is not a PR-first project — as its own genre, distinct from the
workflow doc. **Design-heavy**: the content, where it lives (`CONTRIBUTING.md` vs. a short
README "Development" note), and how it points at `tasks/workflow.md` without pretending to
be it.

## Design

Decisions: a root `CONTRIBUTING.md`; standard PRs welcome.

Create `CONTRIBUTING.md` at the repo root — the functional-contribution guide (its proper
genre: how to build, test, and submit changes), distinct from `tasks/workflow.md` (how the
project is produced). Contents:

- **Getting started** — `cargo build`, `cargo test`, `cargo run -p birch`; toolchain via
  `mise` (pinned `rust = "stable"`).
- **Before submitting** — the gates stay green: `cargo fmt --check`,
  `cargo clippy --all-targets`, `cargo test`; new behavior carries tests.
- **Commits** — Conventional Commits (`<type>(<scope>): <subject>`), per `AGENTS.md`.
- **Pull requests** — welcome, standard flow: fork, branch, PR against `main`; keep the gates
  green and the change focused.
- **Scope** — features are whitelisted: check the scope fence in `docs/design.md` (the feature
  whitelist and the permanent out-of-scope list) before proposing a feature, so a PR is not
  rejected as out of scope. Bug fixes, docs, and adapters are always welcome.
- **How this project is developed (context, not a requirement)** — the maintainer develops
  birch through the sprint workflow in [`tasks/workflow.md`](tasks/workflow.md), driven by the
  in-repo task and sprint markdown files under `tasks/`. Contributors do **not** need to
  follow that flow or touch those files: open GitHub issues and pull requests in the normal
  way. The in-repo workflow is the maintainer's process; the contributor path is standard
  GitHub.

**Reconcile the "no PRs" wording.** `AGENTS.md` and `workflow.md` say development runs "in
sprints (no PRs)" — that describes the *internal* maintainer+agent flow, not external
contribution. Add a short clarifying clause where that phrase appears so the two are not read
as contradictory: internal development is PR-less; external contributions come as standard
issues and PRs. (This AGENTS.md touch overlaps `split-workflow-doc`'s hook edit — apply once,
consistently.)

**Verification.** `CONTRIBUTING.md` renders on GitHub with a Contributing link; the build/test
commands are accurate; links to `docs/design.md` and `tasks/workflow.md` resolve.

