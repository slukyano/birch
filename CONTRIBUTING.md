# Contributing to birch

Bug reports, documentation, host adapters, and code changes to birch are all welcome. Issues and
pull requests are the way in — see [Pull requests](#pull-requests).

## Getting started

birch is a Rust workspace (edition 2024). The toolchain is pinned with
[mise](https://mise.jdx.dev/) (`rust = "stable"`).

```bash
cargo build                 # build all workspace crates
cargo test                  # run tests
cargo run -p birch          # run the TUI
```

## Repository map

- `crates/` — the Rust workspace:
  - `birch-core` — real tree, sources-as-delta-streams, watcher, git status, search, file ops.
    Builds **without** ratatui (the crate boundary enforces the real-tree/render split).
  - `birch-tui` — the render layer (compaction, badges, widget, mouse, context menu).
  - `birch` — the binary: wiring, flags, the socket server, and the `birch ctl` control client.
- `contrib/` — reference host adapters (`birch-tmux`, `birch-herdr`, `birch-cmux`).
- `docs/` — the documentation bundle: [`docs/index.md`](docs/index.md) → `design.md` (product and
  architecture spec), `integrations.md`, and `adr/` (Architecture Decision Records).
- `backlog/` — the maintainer's task/sprint bundle (see [below](#how-this-project-is-developed)).

## Before submitting

Keep the gates green — they are the project's bar (CI runs them on Linux and macOS):

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

New behavior should carry tests.

## Commits

birch uses [Conventional Commits](https://www.conventionalcommits.org/):
`<type>(<scope>): <subject>` — types `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`,
`style`; scope by crate or component (`core`, `tui`, `ctl`, …), omitted for cross-cutting changes.

## Changelog

User-visible changes get a [`CHANGELOG.md`](CHANGELOG.md) entry under the matching
[Keep a Changelog](https://keepachangelog.com/) section (`Added` / `Changed` / `Fixed` /
`Removed`). Internal-only work (refactors, tests, CI) does not. The release tooling reads
`CHANGELOG.md` for GitHub Release notes, so keep the `Unreleased` section current with each PR.

## Documentation

Keep the docs current with the change:

- **`docs/design.md`** — the product and architecture spec; update when scope or a load-bearing
  boundary changes (it is binding — read it before designing a feature).
- **`docs/adr/`** — add an ADR (`NNNN-short-slug.md`) for a decision of architectural weight and
  list it in `docs/index.md`.
- **`docs/integrations.md`** — the host-adapter / integration guide.
- **`README.md`** — user-facing usage and install.

## Pull requests

Pull requests are welcome. Fork, branch, and open a PR against `main`. Keep the change focused and
the gates green.

## How this project is developed

The maintainer develops birch through the sprint workflow described in
[`backlog/workflow.md`](backlog/workflow.md), driven by the task and sprint markdown files under
[`backlog/`](backlog/). That is the maintainer's internal process — contributors do not need to
follow it or touch those files. The contributor path is standard GitHub: issues and pull requests.
