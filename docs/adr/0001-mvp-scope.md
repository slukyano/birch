---
type: ADR
title: The MVP is phases 0.1–0.4; file operations and content search come after
status: Accepted
---

# Context

The design doc sequences work in phases 0.1–0.6 plus "Later", explicitly deferring scope
cuts. A release-worthy first milestone (the MVP) needs a boundary: which backlog tasks are
enough for birch to be viable as a persistent side-pane file tree?

The design doc's own cut principle: **pane integration beats features** — "a birch with
flawless herdr/nvim reveal and no content search beats the reverse". Viability for the
target use means: see the tree with git state, navigate it, open files in the main pane,
find files, and integrate with a pane host. Mutating files and searching file contents both
have everyday fallbacks (the shell, the editor's own search) and sit after integration in
the sequencing.

# Decision

The MVP is exactly phases 0.1–0.4, i.e. these backlog tasks:

- `verify-name-availability`
- `build-core-tree-view` (0.1)
- `add-git-status`, `add-live-updates`, `add-compact-folders` (0.2)
- `add-fuzzy-filename-search` (0.3)
- `add-control-socket`, `add-picker-mode`, `add-state-persistence`,
  `add-host-adapter-and-recipes` (0.4)

Post-MVP, in rough order: `add-file-operations` (0.5), `add-content-search` (0.6),
`add-config-file`, then the "Later" pool (`add-git-changes-source`,
`add-project-view-source`, `add-open-with`).

# Consequences

- The MVP ships read-only: no rename/delete/new, no context menu (the menu arrives with
  file operations, its primary content), no content search, no config file. Runtime
  configuration until then is flags + `birch-ctl set`.
- The adoption funnel (picker → persistent pane) and the integration story are complete at
  MVP; the socket protocol and `--open-cmd` contract are exercised before any packaged
  release hardens them.
- Copy name/path is sequenced with the context menu (0.5) and thus post-MVP; `birch-ctl
  get-path` covers the scripting need at MVP.
