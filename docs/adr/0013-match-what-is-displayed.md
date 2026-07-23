---
type: ADR
title: Search matches what is displayed — names first, path on demand, characters lit
status: Accepted
sprint: sprint-006
---

# Context

ADR 0009's engine matched queries against full relative paths (the fzf/Helix model).
That model is coherent when the list displays paths — but birch's tree displays simple
names, so a match could hinge on characters inside invisible path segments, which read
as wrong results. First use confirmed it. IDEA's tree search — the reference the design
doc leans on — matches visible names only and highlights the matched fragments.

# Decision

- **The match corpus is the entry's simple name.** A query containing `/` switches to
  full relative-path matching — an explicit, visible escape hatch for duplicated
  basenames. The policy is identical in tree jump mode and the picker filter; the two
  render policies of ADR 0009 stay purely about presentation.
- **Matched characters are highlighted** in the displayed string (name in the tree,
  relative path in the picker). Path-mode tree matches highlight the whole name instead
  — the matched characters may not be on screen.
- Camel-hump behavior is provided by nucleo's smart-case (uppercase anchors to
  capitals) plus word-boundary bonuses; no bespoke matcher.

This supersedes ADR 0009's match-against-paths sentence; everything else there (index
worker, nucleo, one engine / two render policies, reveal) stands.

# Consequences

- No more matches explained only by invisible characters; what lights up is why it
  matched.
- Basename duplicates rank as peers until the query grows a `/`; jump mode's tree
  context usually disambiguates visually anyway.
- The index carries the precomputed name and its offset so highlight indices map onto
  either displayed form without recomputation.
