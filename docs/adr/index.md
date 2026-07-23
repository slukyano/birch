---
okf_version: "0.1"
---

# Architecture Decision Records

An OKF bundle: one `type: ADR` concept per decision, named `NNNN-short-slug.md`. Statuses:
`Proposed` → `Accepted` (maintainer-only) | `Rejected`; reversals use a new ADR and `Superseded`.
Process: see [the task workflow](../../tasks/workflow.md#adrs).

# Accepted

* [0001 — The MVP is phases 0.1–0.4; file operations and content search come after](0001-mvp-scope.md)
* [0002 — Keep the name "birch"; Homebrew is the distribution channel of record](0002-keep-the-name-birch.md)
* [0003 — The view-model (flattening, selection, visibility) lives in birch-tui](0003-view-model-lives-in-birch-tui.md)
* [0004 — Sources run on worker threads and feed one app-event channel](0004-sources-run-on-threads.md)
* [0005 — Git state is a side-table snapshot fed by the git CLI, not gix](0005-git-status-via-git-cli.md)
* [0006 — Directory snapshots are a delta; sources stay stateless; the tree reconciles](0006-snapshot-deltas-stateless-sources.md)
* [0007 — Compact chains form via bounded peek-loading of only-child dirs](0007-compaction-peek-loading.md)
* [0008 — Printable characters win — q types into search; Ctrl-C quits](0008-q-types-into-search.md)
* [0009 — One search engine — an ignore-walk index matched by nucleo, rendered two ways](0009-search-index-and-engine.md)
* [0010 — Socket addressing, rendezvous, and lifecycle decisions](0010-socket-addressing-and-lifecycle.md)
* [0011 — The wire protocol — versioned NDJSON request/response, additive-only](0011-ndjson-protocol.md)
* [0012 — Esc backs out one layer at a time — and quits at the top level](0012-esc-backs-out.md)
* [0013 — Search matches what is displayed — names first, path on demand, characters lit](0013-match-what-is-displayed.md)
* [0014 — Compact chains split on demand — → un-collapses, collapse re-fuses](0014-chains-split-on-demand.md)
* [0015 — Click selects, double-click activates](0015-click-selects-double-click-activates.md)
* [0016 — cmux integrates via the Dock, not a workspace-split adapter](0016-cmux-integrates-via-the-dock.md)
