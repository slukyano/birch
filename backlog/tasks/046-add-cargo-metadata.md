---
type: Task
title: Fill in Cargo package metadata
description: Add repository / homepage / keywords / categories to [workspace.package] for discoverability.
status: Draft
priority: low
---

The workspace `[workspace.package]` carries only `version`, `edition`, and `license`. Add
`repository` (the public GitHub URL), `homepage`, `documentation`, `keywords`, and
`categories` so the crates are discoverable and the metadata is ready if any crate is ever
published. Purely metadata; feeds `decide-crates-io-publish`.

## Design

`repository` and `homepage` already landed (added for cargo-dist in Sprint 013). Remaining: add to
`[workspace.package]` `keywords` (≤ 5 — e.g. `tui`, `terminal`, `file-manager`, `tree`, `ratatui`)
and `categories = ["command-line-utilities"]` (a valid crates.io category). Trivial; readies the
metadata for `047`.
