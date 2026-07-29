---
type: Sprint
title: Navigation & search feel
status: Implementing
branch: sprint/016
tasks:
- 063-search-cycles-in-tree-order
- 062-unify-search-in-pick-mode
- 027-add-picker-filter
- 060-right-arrow-always-advances
- 059-fix-guide-chevron-alignment
---

# Scope rationale

Sprint 015 made the tree *look* right; this one makes it *behave* right. Everything in scope is a
report from actually using birch, and three of the five are the same underlying problem: the picker
and the tree teach two different mental models, and search stepping ignores what is on screen.

`062` and `027` land "shown but not selectable/pickable" **once**, as one concept, instead of two
implementations that drift apart — both also amend the `Enter`-always-picks contract from
[`016-unify-picker`](../archive/016-unify-picker.md). `060` applies the same principle to keys: a key
that sometimes does nothing is a key that stops being trusted. `059` rides along as the one visual
item, because its likely resolution is a documented font recommendation rather than a render change.

No new sources, no new modes, no letter hotkeys; the only new public surface is `027`'s two flags.

# In-scope task ledger

- **`063-search-cycles-in-tree-order`** — *minor (bug), high.* `↑`/`↓` during a search step to the
  next/previous match **in tree order** instead of fuzzy-score order. `search()` returns matches
  sorted by `Reverse(score)`; `cycle_match` walks that vector, so the selection teleports. Score
  still decides the initial selection when the query changes; only stepping becomes positional.
- **`062-unify-search-in-pick-mode`** — *mid, design-heavy, high.* Picker search stops being a
  second interaction model: keep the tree, dim non-matches, keep `→`/`←` live, instead of
  `filter_list_active()` replacing the rows with a flat hit list. Open for design: whether the flat
  list survives as an option, and what `Enter` does on a dimmed non-match.
- **`027-add-picker-filter`** — *mid, design-heavy, medium.* Repeatable glob filter for picker mode
  (`--filter '*.md' --filter '*.txt'`) plus `--filter-mode hide|skip`. Folders stay navigable and
  selectable but are only *pickable* when they match. The filter restricts the corpus, the query
  ranks what is left — tree rows and the search index both respect it.
- **`060-right-arrow-always-advances`** — *mid, design-heavy, medium.* `→` never a silent no-op:
  folder → first child (expanding when collapsed), file → next sibling, last file of a folder →
  the parent's next sibling, nothing only on the tree's last file. Two maintainer calls: whether
  the rule reduces `→` to "expand, then `↓`", and what happens to chain splitting
  ([ADR 0014](../../docs/adr/0014-chains-split-on-demand.md)).
- **`059-fix-guide-chevron-alignment`** — *minor, medium.* Indent guides read as misaligned under
  non-`Mono` Nerd Font PUA glyphs, which render wider than a cell and shift right. birch's geometry
  is already correct; the task decides between a documented font recommendation, a theme guide-glyph
  axis, or explicit acceptance, and records the outcome.

# Ordering / dependencies

- **`063` first** — self-contained, no design fork, and it makes search stepping sane before `062`
  changes what a search *shows*.
- **`062` then `027`** — `062` establishes the dim-non-matches-in-place model; `027`'s `skip` mode is
  that same treatment applied to a glob instead of a query. Designed together, implemented in order.
- **`060`** is independent (`FlatView` only) — any point.
- **`059`** is independent and mostly documentation — any point.

`027`'s only `blocked_by` (`016-unify-picker`) is `Done`; the other four carry no blockers.

# Considered but out of scope

- **`061`, `064`, `065`, `066`, `058`, `055`, `056`** — the sprint-015 theme follow-ups (active
  indent guide, badge placement, `random`, animated gradients, terminal-palette adaptation, tui
  encapsulation, user themes). A coherent set that deserves its own sprint.
- **`029`, `034`, `028`** — file operations, context menu, copy paths: design-doc phase 0.5.
- **`030`, `032`, `033`** — the additional sources; "Later" in the design doc, and `032`/`033` are
  blocked on `030`.
- **`026`** — multiple roots: tree model, persistence keying, socket verbs, watchers, and search
  scoping all at once; needs a dedicated design phase.
- **`035`** — high priority but not agent-executable; requires a live interactive herdr session.
- **`051`** — packaging; needs verification against a real `brew install`.
- **`053`** — off-theme; a rider for a future config/settings sprint.

# Sprint-start action

Scope committed to `main`; branch `sprint/016` cut from it. Design phase opens with `062`, whose
resolution shapes `027`.

# Checklist

- [x] 063-search-cycles-in-tree-order (the "no matches means no selection" half lands with 062)
- [ ] 062-unify-search-in-pick-mode
- [ ] 027-add-picker-filter
- [ ] 060-right-arrow-always-advances
- [ ] 059-fix-guide-chevron-alignment

# Open questions

_(none — the design-phase questions were answered in chat: the flat picker list is deleted; a
directory that fails the search is not selectable; the filter gets no config key and no `ctl` key;
the chevron shape is not chosen around a font defect.)_

# Session log

- Scoped and cut: `063`, `062`, `027`, `060`, plus `059` added at scope approval. Branch
  `sprint/016` cut from `main`.
- Design phase: `063` designed (tree-order permutation). Maintainer reframed the narrowing model —
  dimming disables selection, navigation skips dimmed rows, and the filter applies in tree mode too
  — which became **ADR 0023** and the shared basis for `062` and `027`. `060` settled on
  two-press folder entry with chain splitting keeping `→`. `059` investigated with a new vhs +
  pixel-measurement harness: the ⅓-cell offset is reproduced in the non-`Mono` font and traced to
  **PUA chevron glyphs**, which base-font `▸`/`▾` avoid entirely. The chat protocol gained a
  mandatory closing TLDR block (`workflow.md`).
- Design review round: a directory that fails the search is **not selectable** either, so ADR 0023's
  rule became per-narrowing (the search judges every row; the filter judges files only). The
  selection now **anchors forward** on every rematch — the first match at or after the current
  selection, wrapping — which removed the score-ordered initial selection from `063`. The filter
  gets no config key and no `ctl` key (`067` deleted). `059` dropped the glyph change and was
  re-grounded on the **font files themselves**: cmux embeds unpatched JetBrains Mono plus
  **Symbols Nerd Font**, the primary carries no Nerd Font codepoint, and in the fallback the octicon
  chevron sits +0.42 cell off centre while the guide (drawn by the primary) sits at +0.00. The
  outcome is documentation plus a `Mono` primary family, verified live in cmux.
- Design approved: ADR 0023 `Proposed → Accepted`; the five tasks `Draft → Designed`; the sprint
  `Designing → Implementing`. Design merge to `main`.
