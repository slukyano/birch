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
