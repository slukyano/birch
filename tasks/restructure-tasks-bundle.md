---
type: Task
title: Restructure the tasks bundle — numbering, archive, and sprints subdir
description: Number task slugs (001-, 002-, …), move closed tasks to tasks/archive/, and move sprint files to tasks/sprints/.
status: Draft
priority: medium
---

Three workflow / bundle-layout conventions to adopt and document in `tasks/workflow.md`:

1. **Closed tasks archive.** When a task reaches `Done` or `Dropped`, its file moves to
   `tasks/archive/`, keeping the active `tasks/` directory to open work only.
2. **Numbered task slugs.** Task filenames gain a numeric prefix (`001-…`, `002-…`) so the
   backlog has a stable, sortable identity independent of the title.
3. **Sprints subdirectory.** `sprint-NNN.md` files move to `tasks/sprints/`.

Design-heavy migration: OKF concept names are filename stems, and `blocked_by` and sprint
`tasks:` lists reference them, so renaming (numbering) and moving (archive, sprints/) ripple
through those references, `index.md` links, and the session-start "any active Sprint in
`tasks/`" check. Decide the numbering scheme (creation order? gaps allowed on archive?),
whether archived and numbered concept names stay stable, then update the `workflow.md`
schemas and `AGENTS.md` accordingly. Best done alongside `split-workflow-doc`, since both
edit `workflow.md`.
