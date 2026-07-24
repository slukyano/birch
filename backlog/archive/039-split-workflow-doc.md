---
type: Task
title: Split the workflow doc into operational core and meta
description: Trim tasks/workflow.md to what an agent executes; move rationale and cross-repo meta back to the sprint-workflow skill.
status: Done
priority: high
---

`tasks/workflow.md` currently carries both the operational process (lifecycle, gates,
schemas, chat protocol) and a lot of author-time rationale that really belongs to the
reusable `sprint-workflow` skill. Trim the in-repo copy to the run-time core an agent needs
to execute a sprint here, and move the "why" and cross-repo philosophy to the skill.
**Design-heavy**: draw the run-time vs. author-time line without dropping load-bearing
instructions, and keep `AGENTS.md` §"Development workflow" as the hook that points at it.

## Notes — reformulate the publication-hygiene gate

The current gate (§5, "Publication hygiene") bundles hygiene and style, and frames
"maintainer/agent/we are fine" as an exception to third-person voice. That is muddled: a
role is not a person, so it was never a hygiene exception; and records do not need role terms
at all. Split the gate into two, and state hygiene as a positive invariant so the whitelist
disappears:

**Hygiene (hard gate):**

- **No identifiable individuals** in committed content, except the author/copyright identity
  in an authorship or license capacity. (Roles are not individuals, so they need no carve-out.)
- **No environment leakage** — local paths, credentials, private links, internal hostnames,
  machine-specific artifacts.
- **Claims about other projects are factual and sourced** — state facts, never disparage.

**Voice (style): impersonal and agentless everywhere.** Records (summaries, tickets,
`log.md`, task bodies) and product docs (README, code, `design.md`, ADR decisions) name the
thing, not the actor — nominal/passive constructions ("Approved scope: X", "Implemented: Y"),
no roles, no "we", no second person. Drop "we" entirely.

**The one exception** is `workflow.md`'s governance statements, where the role *is* the
meaning — "only the maintainer approves ADRs", "the agent proposes; the maintainer decides",
"stop and ask if a decision belongs to the maintainer". These define the two-party authority
boundary and cannot go impersonal without deleting it. Roles survive only here, minimized to
the authority statements; the rest of `workflow.md` is impersonal too.

Mechanical check: does the sentence name an actor? If so, is it an authority statement in
`workflow.md`? If not, rewrite it nominally.

## Notes — a formal scope-presentation format for cutting a sprint

Close-out has a rich, mandated presentation format (§6c); scope approval has none — the chat
protocol (§"Asking for approval") only says to list tasks with one-liners. Cutting a sprint
is a real gate (it writes `sprint-NNN.md` to `main` and branches) and deserves a symmetric,
formal format. Define one, parallel to §6c, requiring:

- **Sprint id, theme, and branch** — `sprint-NNN`, a short theme, `sprint/NNN`.
- **In-scope task ledger** — every task as: slug, priority, one-line description, and a
  design-weight flag (trivial vs. design-heavy).
- **Ordering / dependencies** among in-scope tasks (`blocked_by`, natural sequence).
- **Considered but out of scope** — tasks weighed and deferred, each with a one-line why.
- **Scope rationale** — the theme that ties the set together and what is deliberately held back.
- **The sprint-start action requested** — commit `tasks/sprint-NNN.md` (status `Designing`) to
  `main`, then cut `sprint/NNN`.

Presented per the chat protocol (§"Asking for approval"): separator, short summary,
self-contained context, file references, explicit questions.

## Design

Rework `tasks/workflow.md` in place — trim to the operational core, adopt the reformulated
hygiene gate and the scope-presentation format, and document the new bundle layout from
`restructure-tasks-bundle`. Implemented together with that task so the documented conventions
match the actual tree.

**Trim (run-time stays, author-time goes).** Keep what an agent needs to execute a sprint
here: the Task/Sprint/ADR lifecycle and states; the sprint steps (scope → design → implement
→ gates → close-out → merge); the gates; the chat approval protocol; the frontmatter schemas;
the surgical-frontmatter rule; the bundle layout. Remove the author-time rationale (why the
workflow exists, cross-repo philosophy, tooling-neutrality essays) — that lives in the
`sprint-workflow` skill's `references/workflow.md` (the maintainer's environment, outside this
repo). Anything removed that is worth preserving is reflected there; the in-repo deliverable is
the trimmed doc.

**Reformulate the publication-hygiene gate (§5)** per the note above: split into **hygiene**
(no identifiable individuals except the copyright identity; no environment leakage; other-
project claims factual and sourced) and **voice** (impersonal and agentless everywhere; the
sole exception is `workflow.md`'s own governance statements). Drop the maintainer/agent/we
whitelist and drop project "we".

**Add the scope-presentation format** per the note above — a section parallel to §6c
(close-out), listing the fields a sprint-cut ask must include.

**Document the new layout** (schemas + a short layout section): task files `NNN-slug.md` with
the number in the concept name; active in `tasks/`, closed in `tasks/archive/`, sprints in
`tasks/sprints/`; the session-start check looks under `tasks/sprints/`. Schemas stay date-free
(the `created`/`timestamp` fields were already removed).

**Also update `AGENTS.md`** §"Development workflow": keep it the short hook, refresh the layout
references (sprint path, numbered slugs) so it stays accurate.

**Verification.** `workflow.md` reads coherently as an operational doc; every referenced
section/format exists; the `AGENTS.md` hook matches; no date fields reintroduced; the doc's own
prose obeys the reformulated voice rule.

