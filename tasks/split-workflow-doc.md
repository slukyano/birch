---
type: Task
title: Split the workflow doc into operational core and meta
description: Trim tasks/workflow.md to what an agent executes; move rationale and cross-repo meta back to the sprint-workflow skill.
status: Draft
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
