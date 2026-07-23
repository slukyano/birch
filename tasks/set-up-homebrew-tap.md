---
type: Task
title: Set up the Homebrew tap and formula
description: Personal slukyano/homebrew-tap with a birch formula installing the binary and the contrib adapters.
status: Draft
priority: high
---

Homebrew is the distribution channel of record
([ADR 0002](../docs/adr/0002-keep-the-name-birch.md)). Create a personal tap
(`slukyano/homebrew-tap`) with a `birch` formula so `brew install slukyano/tap/birch`
works on any Mac. The formula installs the `birch` and `birch-ctl` binaries and the
`contrib/` adapters (`birch-cmux`, `birch-tmux`, `birch-herdr`) — the piece `cargo
install` cannot deliver. Design: build-from-source vs. install from a release tarball
(couples to `automate-releases`), and how the adapters land on PATH.
