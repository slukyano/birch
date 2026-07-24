# birch

## Repository overview

Lean interactive file tree for the terminal (Rust + ratatui). The product and
architecture spec is `docs/design.md` — read it before designing or
implementing features. It defines the scope fence (an explicit feature
whitelist and a permanently-out-of-scope list) and the load-bearing
architecture boundaries; both are binding.

## Commands

```bash
cargo build                 # build all workspace crates
cargo test                  # run tests
cargo clippy --all-targets  # lint
cargo fmt                   # format (--check to verify)
cargo run -p birch          # run the TUI
```

## Architecture

Cargo workspace; internal crates under `crates/` (`publish = false` for now):

- `birch-core` — real tree, sources-as-delta-streams, watcher, git status,
  search, file ops. **Must build without ratatui** — the crate boundary
  compiler-enforces the real-tree/render split.
- `birch-tui` — render layer: compaction, badges, widget, mouse, context menu,
  inline edit.
- `birch` — the binary: wiring, flags, socket server, and the `birch ctl` control client.

Two boundaries are load-bearing (`docs/design.md`, Architecture section);
everything else is negotiable:

1. **Sources are delta streams.** A source emits tree deltas; the view renders
   deltas. Files is the default source, Content Search the second.
2. **Real tree vs. render layer.** All logic — watcher, git, search, ops,
   socket — speaks real paths. Compaction, dimming, badges, highlighting are
   paint-time transforms. Persisted state keys on real paths.

## Constraints and gotchas

- Scope is a whitelist. Reject features on the out-of-scope list in
  `docs/design.md` (multi-select, verbs/modes, plugins, search-and-replace,
  drag-and-drop, copy/move files, …).
- Printable characters are permanently reserved for fuzzy search — never add
  letter hotkeys. The context menu is the primary surface for actions.
- Watch only expanded dirs; never auto-expand, search, or recursively watch
  ignored dirs.
- No mutation verbs on the control socket; mutations are human-initiated only.
  The socket protocol is a public API: additive-only evolution, clients
  tolerate unknown fields.
- Actions live in one action layer consumed by hotkeys, mouse, context menu,
  and socket alike. Menu-specific logic is a smell.

## Verification

`cargo test`, `cargo clippy --all-targets`, and `cargo fmt --check` must pass
before presenting work.

Live cmux testing never drives the maintainer's own cmux instance: launch a
separate instance (`open -na cmux`) and target it by exporting
`CMUX_SOCKET_PATH` (plus `CMUX_WORKSPACE_ID`/`CMUX_SURFACE_ID` from its
`cmux tree --id-format both`) for every command aimed at it.

## Stack

Rust (stable, pinned project-locally via `mise.toml`), edition 2024. Planned
key dependencies: ratatui (TUI), notify (watcher), gix (git status), and the
ripgrep crates — `grep-searcher`, `grep-regex`, `ignore` — for content search
and gitignore logic.

## Development workflow (sessions + sprints)

The project backlog lives in `backlog/`, an OKF bundle (one markdown concept per file with
YAML frontmatter — one `Task` per backlog item, named `NNN-slug.md`, plus one `Sprint` concept
per sprint). Active tasks live in `backlog/tasks/`, closed ones in `backlog/archive/`, sprint records in
`backlog/sprints/`; ADRs live in `docs/adr/` (also an OKF bundle). Frontmatter is state: edits to
it are surgical — change only the keys being updated, never reformat or round-trip a file to
change one field.

This is the maintainer's internal development flow and uses no pull requests; external
contributions are standard GitHub issues and PRs (see `CONTRIBUTING.md`). At session start,
check `backlog/sprints/` for a `Sprint` concept whose `status` is not `Done`/`Aborted` and resume
it from its branch; otherwise propose a scope from the `Draft` backlog. Scope approval =
committing `backlog/sprints/sprint-NNN.md` to `main` and cutting branch `sprint/NNN`. Then: an
interactive design phase (per-task
`## Design` sections + `Proposed` ADRs; maintainer approval → design merge to `main`), an
autonomous implementation phase (commit throughout; **stop and ask** on any decision that
belongs to the maintainer — never guess), gates (the validation suite + an independent
subagent review of the diff + the publication-hygiene check), a verified sprint summary,
and on maintainer approval the final merge. Task lifecycle: `Draft → Designed → Done`
(+ `Dropped`). **Only the maintainer approves ADRs.** All approvals happen in chat and must
be self-contained. Full workflow: `backlog/workflow.md`.

## Commit convention

Conventional Commits: `<type>(<scope>): <subject>`; types `feat`, `fix`,
`docs`, `refactor`, `test`, `chore`, `perf`, `style`. Scope by crate or
component (`core`, `tui`, `ctl`, `tasks`); omit for cross-cutting changes.
