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
