---
type: Task
title: Verify the name "birch" is available
description: Check crates.io and Homebrew for the name before attachment hardens.
status: Done
priority: high
tags:
- research
- packaging
---

Flagged as an open question in [the design doc](../../docs/design.md): verify `birch` (and
`birch-ctl`) availability on crates.io and Homebrew before the name hardens into docs, the
socket path scheme, and adapter names. If taken, decide on an alternative early.

## Design

Query the public registry APIs (no auth needed): `crates.io/api/v1/crates/<name>` for
`birch`, `birch-ctl`, and fallback candidates (`birch-tree`, `birch-tui`, `birch-core`,
`birchtree`); `formulae.brew.sh/api/formula/<name>.json` and `.../cask/<name>.json` for
Homebrew. Decision criteria: the binary/product name must be free on Homebrew (the
distribution channel that matters for a terminal tool); crates.io matters only for a
hypothetical cargo-install channel since the workspace is `publish = false`. The outcome is
recorded as an ADR plus a `## Findings` section here; if the name cannot be kept, renaming
happens now, before the socket path scheme and adapter names harden.

## Findings

| Name | crates.io | Homebrew |
|---|---|---|
| `birch` | **taken** — unrelated secret-rotation tool, published 2025-11, v0.1.1, ~68 downloads | free (no formula, no cask) |
| `birch-ctl` | free | — |
| `birch-core`, `birch-tui`, `birch-tree`, `birchtree` | free | — |

Decision: keep the name — see
[ADR 0002](../../docs/adr/0002-keep-the-name-birch.md). Homebrew is the distribution channel
of record; if cargo-install distribution ever becomes concrete, the binary crate publishes
as `birch-tree` with a `birch` binary name.
