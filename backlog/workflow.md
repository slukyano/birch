---
type: Process
title: Task workflow
description: How development runs in sessions and sprints — scoping, design, autonomous implementation, and approval gates.
tags:
- meta
- process
---

# Overview

This `backlog/` directory is the project backlog **and** an OKF bundle — one markdown file per
concept, with YAML frontmatter carrying the machine-readable state.

This workflow is how the **maintainer** (the project's human owner) develops birch; the two
roles throughout are the maintainer and the coding **agent**. It is not required of outside
contributors — external contributions are standard GitHub issues and pull requests (see
[`CONTRIBUTING.md`](../CONTRIBUTING.md)).

Development happens in **sprints**, driven interactively in **sessions**:

- A **session** is one sitting with the maintainer: open the repo, tell the agent to start or
  continue development.
- A **sprint** is one batch of tasks taken from scope approval through design and
  implementation to a final merge. A sprint usually spans several sessions.

The bundle holds three concept types:

- `Task` — one backlog item per file, named `NNN-slug.md` (a zero-padded number is part of the
  concept name).
- `Sprint` — one `sprint-NNN.md` per sprint: the durable state of active and past work.
- `Process` — this document.

**Layout.** Active tasks (`Draft`, `Designed`) live in `backlog/tasks/`; closed tasks (`Done`,
`Dropped`) move to `backlog/archive/`; sprint records live in `backlog/sprints/`. `index.md`,
`log.md`, and `workflow.md` stay at the bundle root. `index.md` and `log.md` are OKF-reserved
(a listing and a change log), not concepts. Architecture Decision Records live in the
[`docs/`](../docs/index.md) bundle (the `adr/` subdirectory).

## Editing rule

Frontmatter is state, and edits to it are **surgical**: change only the keys being updated
(status flips), preserve the rest of the file byte-for-byte, and never reformat or
round-trip a whole document to change one field. OKF-aware tooling that guarantees this is
preferred over hand edits; hand edits are legitimate when kept to the same discipline.

# Task lifecycle

```
   Draft ──────► Designed ──────► Done
  (a seed)   (design merged)  (impl merged)      Dropped (terminal, from anywhere)
```

| `status` | Meaning | Set when |
|----------|---------|----------|
| `Draft` | A seed — a few words to a short description. | Authored directly on `main`, anytime, by maintainer or agent. |
| `Designed` | The task body carries an approved design (a `## Design` section). | The sprint's **design merge** lands on `main`. |
| `Done` | Implemented, gated, approved. | The sprint's **final merge** lands on `main`. |
| `Dropped` | Abandoned; the body records why. | Maintainer decision, anytime. |

There are no lock/claim states and no lease fields: sprints are single-flight, and "in a
sprint" is recorded in the sprint concept, not on the task.

# Sprint lifecycle

One sprint moves through:

```
  (scope approved)          (design approved)         (implementation approved)
        │                          │                            │
        ▼                          ▼                            ▼
    Designing ──────────────► Implementing ──────────────────► Done
        │                          │
        └────────── Aborted ◄──────┘   (maintainer decision; record why)
```

## 1. Session start

Every session begins by checking for unfinished work: is there any `Sprint` concept in
`backlog/sprints/` whose `status` is not `Done` or `Aborted`?

- **An active sprint exists** → check out its branch (the branch always has the freshest
  sprint state) and resume from the sprint body: the task checklist, open questions, and
  session log say exactly where work stopped.
- **No active sprint** (including: no sprint concept has ever been created) → propose scope
  for a new one. Also glance at `git branch --list 'sprint/*'` for a stray branch.

## 2. Scoping

The agent reviews the open backlog (`Draft` tasks, unblocked) and proposes a set for the
sprint — proposing *all* open tasks is fine when the scope feels right. The maintainer adjusts and
approves.

**Scope approval is the sprint-start commit on `main`**: create `backlog/sprints/sprint-NNN.md`
(status `Designing`, the task list, the branch name), commit it to `main`, then create the
sprint branch `sprint/NNN` from it. All subsequent work happens on the branch.

The scope is **presented for approval** in the chat protocol (below), with:

- the sprint id, theme, and branch;
- an **in-scope task ledger** — every task as slug, priority, one-line description, and a
  design-weight flag (trivial vs. design-heavy);
- **ordering / dependencies** among the in-scope tasks;
- **considered but out of scope** — tasks weighed and deferred, each with a one-line why;
- the **scope rationale** — what ties the set together and what is deliberately held back;
- the **sprint-start action** requested (commit `backlog/sprints/sprint-NNN.md`, cut `sprint/NNN`).

## 3. Design phase (interactive)

On the sprint branch, the agent and the maintainer design the tasks **one by one**. The maintainer acts
as stakeholder, product owner, and senior architect; the agent drives — proposes a design,
asks questions, records decisions. Per task, the outcome is:

- a `## Design` section written into the task body — the executable plan; and
- zero or more **ADRs** in `docs/adr/` (status `Proposed`) for decisions of architectural
  weight. See [ADRs](#adrs).

Commit throughout the phase. When all tasks in scope are designed, the maintainer reviews the
batch. **Design approval** triggers, in order:

1. ADRs from this phase flip `Proposed → Accepted` (only the maintainer approves ADRs).
2. Tasks flip `Draft → Designed`; the sprint flips `Designing → Implementing`.
3. The sprint branch is **merged to `main`** (design merge). The branch stays alive.

## 4. Implementation phase (autonomous)

The agent implements the designed tasks independently — preferably in one long run, using
subagents where appropriate. Rules of the phase:

- **Commit throughout**, per coherent step, on the sprint branch.
- **Track progress** in the sprint body (per-task checklist), so any session can resume.
- **Stop and ask**: if a decision surfaces that belongs to the maintainer — a product call or an
  architectural fork the design doesn't cover — do **not** guess. Record the open question in
  the sprint body (`## Open questions`), commit, and stop that task (or the sprint, if it
  blocks everything). Fidelity over throughput.
- New decisions of architectural weight get ADRs (`Proposed`) as part of the change.

## 5. Gates (must pass before presenting)

- **Validation** — the full suite passes: `cargo test`, `cargo clippy --all-targets`, and
  `cargo fmt --check`. New behavior is covered by tests.
- **Independent review** — a fresh subagent with no implementation context reviews the full
  sprint diff; findings are fixed (or explicitly presented as known issues).
- **Publication hygiene** — everything committed must be publishable as-is, since the repo
  (history included) is public-bound. Two checks:
  - **Hygiene** — no identifiable individuals except the author/copyright identity in an
    authorship or license capacity; no environment leakage (local paths, credentials, private
    links, internal hostnames, machine-specific artifacts); and claims about other projects are
    factual and sourced (state facts, never disparage).
  - **Voice** — impersonal and agentless: name the thing, not the actor (nominal or passive
    constructions), no second-person or chat-transcript prose, no project "we". The sole
    exception is this document's governance statements, where a role *is* the meaning ("only the
    maintainer approves ADRs"); records, product docs, ADR decisions, and summaries carry no
    roles.

## 6. Close-out, presentation & final merge

Once the gates pass, close the sprint out **on the branch** so the maintainer reviews the exact
state that will merge — bookkeeping included. The only thing gated purely on approval is the
merge itself.

### 6a. Close-out bookkeeping (committed to the branch, before presenting)

1. Flip every delivered task `Designed → Done` (surgical frontmatter edit: `status` only) and
   move its file into `backlog/archive/`.
2. Flip the sprint `Implementing → Done`, the same way.
3. Write a `## Sprint summary` into the sprint body and a close-out `## Session log` line.
4. Bring the hand-maintained `backlog/index.md` and `backlog/log.md` current (move Done tasks to a
   `# Done` section, linking their `backlog/archive/` path; mark the sprint Done; add a **dated**
   log entry — `backlog/log.md` groups entries under a `## YYYY-MM-DD` heading).
5. **Every open question / deferred idea must have a home.** If something was left undone —
   deliberately or by omission — it is either done now or captured as a `Draft` task. Never
   say "carried to the backlog" without a concrete task name; create the task if none exists.
6. Commit the bookkeeping (`chore(sprint): close out sprint-NNN ...`).

### 6b. The summary artifact + independent accuracy check

Write the full review to a **file** (e.g. `.scratchpad/sprint-NNN-review.md`), then have a
**fresh reviewer subagent** (no implementation context) verify it against the real diff
(`git diff main..sprint/NNN`) and the live gate output — commit/file counts, test numbers,
task states, and every "what changed" / "bug fixed" / "limitation" claim. Fix any inaccuracy
the reviewer finds **before** presenting. Do not present an unverified summary.

### 6c. Presentation format (always include)

Present in the chat protocol below. The summary MUST include:

- **A task ledger listing *every* task involved** — Done, Dropped, created-this-sprint, and
  planned-but-descoped. For each: its **relative weight** (`major` / `mid` / `minor` — size AND
  importance AND future impact; e.g. a package rename is a small diff but major impact), and a
  **⚠️ mark if it transformed significantly** during design or implementation.
- **Per change: what changed and why, and how the task transformed** from its original framing.
- **Explicitly what was NOT done** — deliberately or by omission — each item paired with its
  disposition (done, or the **named** `Draft` task that now holds it).
- **Breaking changes.**
- **Architectural decisions** made, with their ADRs.
- **Bugs found & fixed** (review findings & how they were resolved) — its own section.
- **Remaining limitations & highlights** — a separate, clearly-flagged must-read section
  (sharp edges, deliberate trade-offs, things a user will trip on), never folded into prose.

Whenever the summary says something was deferred or carried forward, **name the backlog task
that holds it** — never a bare "added to the backlog".

### 6d. Final merge

**Implementation approval** triggers, in order:

1. New ADRs flip `Proposed → Accepted` (or are revised/rejected per the maintainer).
2. The sprint branch is **merged to `main`** (final merge, `--no-ff`) and deleted.

(Task/sprint status flips already happened in 6a; if the maintainer sends changes back, revert or
adjust the bookkeeping before merging.) Tasks that didn't make it stay `Designed` (or return to
`Draft` if the design was invalidated) and go back to the backlog for a future sprint.

# Asking for approval (chat protocol)

All approvals happen **in the chat**. The maintainer decides from what's presented there — files
are for double-clicking into details, never required reading for a decision. Whenever the
agent finishes an iteration or needs a decision, it formats the ask as:

1. a **separator** (`---`),
2. the **question or summary in short** — one or two sentences,
3. the **complete decision context** — everything needed to decide, self-contained in the
   chat (quote the relevant parts; never just point at files, never dump whole files). A
   batch approval that covers tasks (scope, design, implementation) lists **every task in
   the batch with at least a one-line description** — never a bare task name, never "as
   presented before",
4. **references to the key files** touched or decided on (paths, with line numbers where it
   helps), so the maintainer can double-click into any detail,
5. the **explicit list of questions** to answer (or the single question), each answerable
   with a short reply.

This applies to scope approval, design approval, implementation approval, ADR acceptance,
and stop-and-ask questions raised mid-implementation.

# Sprint frontmatter schema

| Field | Required | Type | Notes |
|-------|----------|------|-------|
| `type` | yes | string | Always `Sprint`. |
| `title` | yes | string | Short theme, e.g. "CLI ergonomics". |
| `status` | yes | string | `Designing` \| `Implementing` \| `Done` \| `Aborted`. |
| `branch` | yes | string | The sprint branch, e.g. `sprint/001`. |
| `tasks` | yes | list | concept names of the tasks in scope. |

The **body** is the working state: scope rationale, a per-task checklist (`[ ]` → `[x]`) kept
current during implementation, `## Open questions` (the stop-and-ask log), and a short
`## Session log` (one line per session: what moved).

# Task frontmatter schema

| Field | Required | Type | Notes |
|-------|----------|------|-------|
| `type` | yes | string | Always `Task`. |
| `title` | yes | string | Short imperative title. |
| `description` | recommended | string | One-line summary; also used in `index.md`. |
| `status` | yes | string | `Draft` \| `Designed` \| `Done` \| `Dropped`. |
| `priority` | recommended | string | `low` \| `medium` \| `high`. |
| `tags` | optional | list | Cross-cutting labels (`research`, `cli`, `packaging`, …). |
| `blocked_by` | optional | list | concept names of tasks that must be `Done` first. |

Producers may add more keys — the schemas are open, and OKF tooling treats unknown keys as
ordinary columns. A missing `blocked_by` means unblocked; don't write empty lists.

## Blockers

`blocked_by` lists the **concept names** of prerequisite tasks (a concept name is the filename
without `.md` — `NNN-slug`, resolved wherever the file lives in the bundle). A task is eligible
for a sprint when its `status` is `Draft` (or `Designed`,
for implementation) and every task in `blocked_by` is `Done`. A dangling or mistyped blocker
counts as blocking (conservative — better to stall than to double-build). Tasks within one
sprint may depend on each other; the design phase orders them.

# ADRs

Architecture Decision Records live in [`docs/adr/`](../docs/adr/) — itself an OKF bundle, one
concept per decision, named `NNNN-short-slug.md`.

| Field | Required | Type | Notes |
|-------|----------|------|-------|
| `type` | yes | string | Always `ADR`. |
| `title` | yes | string | The decision, stated as a decision. |
| `status` | yes | string | `Proposed` \| `Accepted` \| `Rejected` \| `Superseded`. |
| `sprint` | optional | string | concept name of the originating sprint. |
| `superseded_by` | when superseded | string | concept name of the replacing ADR. |

Body: Context, Decision, Consequences (and Alternatives considered, when useful).

Rules:

- The agent **proposes** ADRs — during design sessions and during implementation — as part of
  the change itself, committed on the sprint branch.
- **Only the maintainer approves ADRs.** `Proposed → Accepted` happens at the batch approval
  (design or implementation), never unilaterally.
- Reversing an accepted decision means a new ADR that supersedes the old one, not an edit.
