# birch

Lean interactive file tree for the terminal (Rust + ratatui).

- **What birch is, usage, install:** [`README.md`](README.md).
- **Dev basics — build/test commands, repository map, conventions, changelog, docs upkeep:**
  [`CONTRIBUTING.md`](CONTRIBUTING.md).
- **Product & architecture spec (binding):** [`docs/design.md`](docs/design.md) — read it before
  designing or implementing a feature. It defines the scope fence (an explicit feature whitelist
  and a permanent out-of-scope list) and the load-bearing boundaries.

This file holds the rules an agent must follow that are not already in those docs.

## Load-bearing boundaries

Two boundaries (`docs/design.md`, Architecture) are load-bearing; everything else is negotiable:

1. **Sources are delta streams.** A source emits tree deltas; the view renders deltas. Files is
   the default source, Content Search the second.
2. **Real tree vs. render layer.** All logic — watcher, git, search, ops, socket — speaks real
   paths. Compaction, dimming, badges, highlighting are paint-time transforms; persisted state
   keys on real paths.

`birch-core` **must build without ratatui** — the crate boundary compiler-enforces the split.

## Constraints and gotchas

- Scope is a whitelist. Reject features on the out-of-scope list in `docs/design.md` (multi-select,
  verbs/modes, plugins, search-and-replace, drag-and-drop, copy/move files, …).
- Printable characters are permanently reserved for fuzzy search — never add letter hotkeys. The
  context menu is the primary surface for actions.
- Watch only expanded dirs; never auto-expand, search, or recursively watch ignored dirs.
- No mutation verbs on the control socket; mutations are human-initiated only. The socket protocol
  is a public API: additive-only evolution, clients tolerate unknown fields.
- Actions live in one action layer consumed by hotkeys, mouse, context menu, and socket alike.
  Menu-specific logic is a smell.

## Verification

The gates (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — see
`CONTRIBUTING.md`) must pass before presenting work.

Live cmux testing never drives the maintainer's own cmux instance: launch a separate instance
(`open -na cmux`) and target it by exporting `CMUX_SOCKET_PATH` (plus
`CMUX_WORKSPACE_ID`/`CMUX_SURFACE_ID` from its `cmux tree --id-format both`) for every command
aimed at it.

## Development workflow (sessions + sprints)

**Follow this workflow only when asked to develop birch as the maintainer** — otherwise it is
context, not a requirement, and external contributions go through standard GitHub issues and PRs
(see `CONTRIBUTING.md`).

The backlog lives in `backlog/`, an OKF bundle (one markdown concept per file with YAML
frontmatter — one `Task` per item, named `NNN-slug.md`, plus one `Sprint` per sprint). Active
tasks live in `backlog/tasks/`, closed ones in `backlog/archive/`, sprint records in
`backlog/sprints/`; ADRs live in the `docs/` bundle (`adr/`). Frontmatter is state: edits are
surgical — change only the keys being updated, never reformat or round-trip a file.

At session start, check `backlog/sprints/` for a `Sprint` whose `status` is not `Done`/`Aborted`
and resume it from its branch; otherwise propose a scope from the `Draft` backlog. Scope approval =
committing `backlog/sprints/sprint-NNN.md` to `main` and cutting branch `sprint/NNN`. Then: an
interactive design phase (per-task `## Design` sections + `Proposed` ADRs; maintainer approval →
design merge to `main`), an autonomous implementation phase (commit throughout; **stop and ask**
on any decision that belongs to the maintainer — never guess), gates (validation + an independent
subagent review of the diff + the publication-hygiene check), a verified sprint summary, and on
maintainer approval the final merge. Task lifecycle: `Draft → Designed → Done` (+ `Dropped`).
**Only the maintainer approves ADRs.** All approvals happen in chat and must be self-contained.
Full workflow: [`backlog/workflow.md`](backlog/workflow.md).
