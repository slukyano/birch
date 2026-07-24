# Contributing to birch

Bug reports, documentation, host adapters, and code changes to birch are all welcome.

## Getting started

birch is a Rust workspace (edition 2024). The toolchain is pinned with
[mise](https://mise.jdx.dev/) (`rust = "stable"`).

```bash
cargo build                 # build all workspace crates
cargo test                  # run tests
cargo run -p birch          # run the TUI
```

## Before submitting

Keep the gates green — they are the project's bar:

```bash
cargo fmt --check
cargo clippy --all-targets
cargo test
```

New behavior should carry tests.

## Commits

birch uses [Conventional Commits](https://www.conventionalcommits.org/):
`<type>(<scope>): <subject>` — types `feat`, `fix`, `docs`, `refactor`, `test`, `chore`,
`perf`, `style`; scope by crate or component (`core`, `tui`, `ctl`, …), omitted for
cross-cutting changes.

## Pull requests

Pull requests are welcome. Fork, branch, and open a PR against `main`. Keep the change
focused and the gates green.

## Scope

birch has a deliberately small, whitelisted feature set. Before proposing a feature, check
the scope fence in [`docs/design.md`](docs/design.md) — it lists both the accepted features
and the permanently out-of-scope ones — so a change is not turned down for being out of
scope. Bug fixes, documentation, and host adapters are always welcome.

## How this project is developed

The maintainer develops birch through the sprint workflow described in
[`tasks/workflow.md`](tasks/workflow.md), driven by the task and sprint markdown files under
[`tasks/`](tasks/). That is the maintainer's internal process — contributors do not need to
follow it or touch those files. The contributor path is standard GitHub: issues and pull
requests.
