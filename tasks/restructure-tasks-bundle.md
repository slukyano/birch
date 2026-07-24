---
type: Task
title: Restructure the tasks bundle — numbering, archive, and sprints subdir
description: Number task slugs (001-, 002-, …), move closed tasks to tasks/archive/, and move sprint files to tasks/sprints/.
status: Designed
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

## Design

Decisions: numbered slugs `NNN-slug` (the number is part of the filename *and* the concept
name); the whole existing backlog renumbered now.

**Layout.** Within the `tasks/` bundle:

- `tasks/NNN-slug.md` — active tasks (`Draft`, `Designed`).
- `tasks/archive/NNN-slug.md` — closed tasks (`Done`, `Dropped`).
- `tasks/sprints/sprint-NNN.md` — sprint records (already numbered).
- `tasks/index.md`, `tasks/log.md`, `tasks/workflow.md` — stay at the bundle root.

Concept names remain filename stems, resolved across subdirectories, so `blocked_by` and
sprint `tasks:` entries resolve whether a task is active or archived; only `index.md` links
carry the path.

**Numbering.** Three digits, zero-padded, dense (no gaps). Assignment order — deterministic,
generated at implementation, and identity-order, *not* true creation order (unrecoverable:
dates stripped, history squashed):

1. Delivered tasks, grouped by delivering sprint in order `sprint-001 → sprint-011`, and
   within a sprint in that sprint's `tasks:` list order.
2. Then currently-open tasks in `index.md` order (the product backlog, then the
   `# Publication` batch).
3. New tasks continue the sequence at creation / close-out.

**Migration — one atomic change on the branch:**

1. Rename every task file to `NNN-slug.md`; move `Done`/`Dropped` files into `tasks/archive/`.
2. Move `sprint-*.md` into `tasks/sprints/`.
3. Rewrite every `blocked_by` entry and every sprint `tasks:` entry to the numbered concept
   name — including `sprint-012.md`'s own list, and blockers that point at now-archived tasks.
4. Update all `index.md` links to the new paths/names; fix any file links in `log.md`.
5. `workflow.md`'s layout/schema text is rewritten by `split-workflow-doc` so the documented
   conventions match the new tree.

**Verification.** Every `blocked_by` / `tasks:` reference resolves to an existing concept; no
dangling `index.md` links; the session-start check finds sprints under `tasks/sprints/`. No
Rust changes.

