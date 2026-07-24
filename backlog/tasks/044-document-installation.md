---
type: Task
title: Document installation in the README
description: An Install section covering brew install and cargo install --git, noting the adapter caveat.
status: Designed
priority: medium
blocked_by:
- 042-set-up-homebrew-tap
---

The README has a "Building" section but no "Install" section. Add one covering
`brew install slukyano/tap/birch` (the recommended path) and `cargo install --git
https://github.com/slukyano/birch birch` for Rust users — flagging that `cargo install`
delivers only the binaries, not the `contrib/` adapters. Blocked on the tap existing so
the documented command is real.

## Design

Add an **Install** section to the README (near the top, per 049's structure), in order:

- `brew install slukyano/tap/birch` — the recommended path (the tap is live; ADR 0018).
- `cargo install --git https://github.com/slukyano/birch birch` — for Rust users.
- Build from source — `cargo build`, `cargo run -p birch`.

Note that `cargo install` delivers only the binary (no adapters), and that a brew install puts the
adapters in `$(brew --prefix)/share/birch/`, not on `PATH` (see `051-install-adapters-on-path`).
