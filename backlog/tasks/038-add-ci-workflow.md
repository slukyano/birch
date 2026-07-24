---
type: Task
title: Add the CI workflow
description: GitHub Actions running fmt --check, clippy --all-targets, and test on push and PR.
status: Draft
priority: high
---

The project's own gates (`cargo fmt --check`, `cargo clippy --all-targets`, `cargo test`)
are not enforced on GitHub. Add `.github/workflows/ci.yml` running them on push and pull
request against `main`, with the toolchain pinned via `mise.toml` (`rust = "stable"`).
Design: caching, whether to matrix over OSes, and the MSRV stance.

## Design

Add `.github/workflows/ci.yml`, triggered on `push` to `main` and on `pull_request`. Enforce the
gates on **Linux and macOS**.

- **Matrix:** a `test` job on `[ubuntu-latest, macos-latest]` — checkout, install stable Rust with
  `rustfmt` + `clippy` (`dtolnay/rust-toolchain@stable` or `actions-rust-lang/setup-rust-toolchain`),
  cache with `Swatinem/rust-cache`, then `cargo clippy --all-targets -- -D warnings` and
  `cargo test`. A single `fmt` job on `ubuntu-latest` runs `cargo fmt --check` (formatting is
  platform-agnostic, so once suffices).
- **Toolchain:** stable. Edition 2024 needs Rust ≥ 1.85; no separate MSRV job (the project tracks
  stable via `mise`) — one can be added later if a support floor is promised.
- **Concurrency:** cancel superseded runs per ref.

No source changes; the gates already pass locally. Verification: the workflow runs green on a pull
request and on `main`.

