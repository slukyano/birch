# birch

Modern interactive file tree for the terminal (Rust + ratatui).

- **What birch is, usage, install:** [`README.md`](README.md).
- **Dev basics — build/test commands, repository map, conventions, changelog, docs upkeep:**
  [`CONTRIBUTING.md`](CONTRIBUTING.md).
- **Product & architecture spec (binding):** [`docs/design.md`](docs/design.md) — read it before
  designing or implementing a feature. It defines the scope fence (the identity sentence, judged in
  two halves) and the load-bearing boundaries.

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

- Scope is the identity, judged — there is no list to look a feature up in. Admit one only if the
  IDEA / VS Code explorer panel has it **and** it survives a narrow terminal pane where every
  printable belongs to search: both halves of the identity sentence in `docs/design.md`. A different
  shape — miller columns, a verb language, vim modes — fails whichever neighbouring tool has it.
  Prohibitions a section of `docs/design.md` states for itself (no plugins, no search-and-replace,
  no mutation verbs on the socket) stand on their own terms. Unclear cases go to the maintainer;
  "another file manager has it" is not an argument.
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

Live cmux testing never drives the maintainer's own cmux instance. `open -na cmux` does **not**
give you a second one — cmux allows a single instance per bundle identifier and the duplicate exits
silently during startup. Use the throwaway instance driven by
[`contrib/testcmux`](contrib/testcmux), which runs a re-identified, ad-hoc-signed copy of the
bundle on its own socket and its own UserDefaults, and target it with `--socket` (or
`CMUX_SOCKET_PATH`) on every command.

## Publishing

**Nothing reaches `main` or the outside world without explicit approval, however small.** Local
commits and pushes to a sprint branch need no permission and should happen freely; everything
below is asked for first, every time, including a one-line documentation fix:

- pushing to `main` — whether directly or by merging a branch into it;
- tags, releases, and anything published to the Homebrew tap;
- issues, comments, or pull requests on any other project's tracker.

Approval is per action, not standing: approval to push once is not approval to push again.

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
