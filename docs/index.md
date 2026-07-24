---
okf_version: "0.1"
---

# Documentation

The `docs/` OKF bundle: the product and architecture spec, the integration guide, and the
Architecture Decision Records (`adr/`, one `type: ADR` concept per decision).

# Docs

* [Design — product and architecture spec](design.md) - The scope fence and load-bearing boundaries; read before designing or implementing.
* [Integrations](integrations.md) - Integrating birch into a pane host: the socket protocol, adapters, and editor recipes.

# ADRs

ADR concepts live in [`adr/`](adr/), named `NNNN-short-slug.md`. Statuses: `Proposed` →
`Accepted` (maintainer-only) | `Rejected`; reversals use a new ADR and `Superseded`.
Process: see [the task workflow](../backlog/workflow.md#adrs).

## Accepted

* [0001 — The MVP is phases 0.1–0.4; file operations and content search come after](adr/0001-mvp-scope.md)
* [0002 — Keep the name "birch"; Homebrew is the distribution channel of record](adr/0002-keep-the-name-birch.md)
* [0003 — The view-model (flattening, selection, visibility) lives in birch-tui](adr/0003-view-model-lives-in-birch-tui.md)
* [0004 — Sources run on worker threads and feed one app-event channel](adr/0004-sources-run-on-threads.md)
* [0005 — Git state is a side-table snapshot fed by the git CLI, not gix](adr/0005-git-status-via-git-cli.md)
* [0006 — Directory snapshots are a delta; sources stay stateless; the tree reconciles](adr/0006-snapshot-deltas-stateless-sources.md)
* [0007 — Compact chains form via bounded peek-loading of only-child dirs](adr/0007-compaction-peek-loading.md)
* [0008 — Printable characters win — q types into search; Ctrl-C quits](adr/0008-q-types-into-search.md)
* [0009 — One search engine — an ignore-walk index matched by nucleo, rendered two ways](adr/0009-search-index-and-engine.md)
* [0010 — Socket addressing, rendezvous, and lifecycle decisions](adr/0010-socket-addressing-and-lifecycle.md)
* [0011 — The wire protocol — versioned NDJSON request/response, additive-only](adr/0011-ndjson-protocol.md)
* [0012 — Esc backs out one layer at a time — and quits at the top level](adr/0012-esc-backs-out.md)
* [0013 — Search matches what is displayed — names first, path on demand, characters lit](adr/0013-match-what-is-displayed.md)
* [0014 — Compact chains split on demand — → un-collapses, collapse re-fuses](adr/0014-chains-split-on-demand.md)
* [0015 — Click selects, double-click activates](adr/0015-click-selects-double-click-activates.md)
* [0016 — cmux integrates via the Dock, not a workspace-split adapter](adr/0016-cmux-integrates-via-the-dock.md)
* [0018 — Release automation and the Homebrew tap via cargo-dist](adr/0018-release-via-cargo-dist.md) (supersedes 0017)
* [0019 — The control client is a `birch ctl` subcommand, not a separate binary](adr/0019-control-client-is-a-birch-subcommand.md)

## Superseded

* [0017 — Distribute prebuilt binaries via a Homebrew tap, built on tag](adr/0017-prebuilt-binaries-and-homebrew-tap.md) (superseded by 0018)

