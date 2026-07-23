---
type: Task
title: Match what is displayed — name-first search with highlighted characters
description: Search matches simple names by default (path only when the query has a /), identically in tree and picker; matched characters highlight.
status: Done
priority: high
---

Maintainer feedback on the MVP search: matching against full relative paths produced
"weird" hits — letters matched inside path segments the tree does not display. The
reference behavior (IDEA's tree search) matches visible names only and highlights the
matched fragments. The policy must be identical in tree and picker mode.

## Design

Per [ADR 0013](../docs/adr/0013-match-what-is-displayed.md):

- **Corpus**: the entry's simple name (basename). A query containing `/` switches that
  query to full relative-path matching — the explicit escape hatch for duplicated names
  (`mod.rs`, `index.ts`). The same rule applies in tree jump mode and the picker filter;
  only rendering differs, as ADR 0009 already established.
- **Camel humps**: free from nucleo — smart-case makes an uppercase query char anchor to
  capitals (`FB` finds `FooBar`), and boundary bonuses rank word-initial matches first.
- **Highlighting**: `search()` returns matched character indices per entry (nucleo's
  `indices` API). Tree rows highlight the matched characters inside the displayed name
  (chain rows: indices offset into the label's tail segment); the picker highlights them
  inside the displayed relative path (name-mode indices shifted by the dir-prefix
  length). Path-mode matches in the tree fall back to whole-name highlight (the matched
  characters may live in segments the row does not show). Non-matches keep dimming.
- `IndexEntry` carries the precomputed name and its offset within `rel`;
  `SearchState.matched_set` becomes a path → indices map.
