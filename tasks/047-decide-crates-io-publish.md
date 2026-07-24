---
type: Task
title: Decide the crates.io publishing story
description: Evaluate publishing the binary crate as birch-tree (ADR 0002 fallback) vs staying Homebrew-only.
status: Draft
priority: low
blocked_by:
- 046-add-cargo-metadata
---

All crates are `publish = false` and the crates.io name `birch` is taken
([ADR 0002](../docs/adr/0002-keep-the-name-birch.md)). Decide whether to publish the
binary crate as `birch-tree` with `[[bin]] name = "birch"` (enabling `cargo install
birch-tree`) or to stay Homebrew-only. The outcome is a decision, likely a new ADR, with
little or no code beyond metadata.
