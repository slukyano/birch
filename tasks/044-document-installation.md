---
type: Task
title: Document installation in the README
description: An Install section covering brew install and cargo install --git, noting the adapter caveat.
status: Draft
priority: medium
blocked_by:
- 042-set-up-homebrew-tap
---

The README has a "Building" section but no "Install" section. Add one covering
`brew install slukyano/tap/birch` (the recommended path) and `cargo install --git
https://github.com/slukyano/birch birch` for Rust users — flagging that `cargo install`
delivers only the binaries, not the `contrib/` adapters. Blocked on the tap existing so
the documented command is real.
