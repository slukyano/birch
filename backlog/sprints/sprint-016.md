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
- [x] 062-unify-search-in-pick-mode
- [x] 027-add-picker-filter
- [x] 060-right-arrow-always-advances
- [x] 059-fix-guide-chevron-alignment

# Open questions

_(none — the design-phase questions were answered in chat: the flat picker list is deleted; a
directory that fails the search is not selectable; the filter gets no config key and no `ctl` key;
the chevron shape is not chosen around a font defect.)_

# Sprint summary

Search and the picker became one thing. **[ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)**
replaced two narrowing behaviours with one rule: a search or a filter **dims** rows instead of
replacing them, and **a dimmed row is inert** — it cannot hold the selection, a click does nothing,
`Enter` never acts on it. Each narrowing declares what it judges: a search judges every row, so a
directory that fails the query is dim too; the glob filter judges files only, so directories stay
navigable and are gated on *pick* alone.

- **`063`** — matches are held in tree order (`sort_tree_order`, keyed on path components as
  `(is_file, lowercased)`, which reproduces how a level is drawn), so `↑`/`↓` travel down and up the
  pane. The selection **anchors forward** on every rematch: the first match at or after the current
  row, wrapping at the end, staying put while the row still matches. Score order lost its last
  consumer; `search()` keeps it anyway, and `rematch` lost its `keep_position` parameter.
- **`062`** — `filter_list_active` and `flat_view::match_rows` are gone, and with them flat rows,
  dead arrows, chevron-clicks-as-name-clicks, and the snap-to-top-match. `Row.search: Option<bool>`
  became `Row.live` + `Row.matched` + `Row.pickable`. `Esc` restores the pre-search view in both
  modes; with nothing live there is no selection and `Enter` reports `no matches`.
- **`027`** — `--filter <glob>` (repeatable) and `--filter-mode hide|skip`, in **both** modes. A
  pattern without `/` matches the name, one with `/` the root-relative path — search's corpus rule.
  `globset` was already in the tree under `ignore`, so brace expansion came free with no new
  transitive crate, and patterns compile before the terminal is taken over. `hide` also drops
  directories *known* to hold nothing; a directory whose listing has not been read is always kept.
- **`060`** — `→` expands a collapsed directory, splits an expanded chain (ADR 0014 intact), or
  advances to the next live row. Inert only where the task allows: no live row follows.
- **`059`** — no code change, by decision. The reported misalignment was traced to font metrics with
  a new measurement harness and then to the font files themselves: cmux embeds an unpatched
  JetBrains Mono (no icon codepoints at all) plus **Symbols Nerd Font**, in which birch's chevrons
  advance 1.67 cells and sit +0.42 cell off centre while the indent guide, drawn by the primary
  font, sits at +0.00. Setting the primary family to a `Mono` build fixes it; documented in the
  README and `docs/research/nerd-font-glyphs.md`, with the upstream Ghostty reports.

Six commits, 21 files, +1348/−258. Tests went from 144 on `main` to **161** (18 added, 1 removed
with the flat picker list); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check`
clean.

**Bugs found and fixed** (both surfaced by rendering the change and looking at it, neither caught by
a unit test):

1. The **selection wash swallowed the match highlight**. `draw` painted the edge-to-edge wash with
   `set_style` over the whole row rect, overwriting the background of every cell — including the
   gold `match_bg` of lit characters, whose near-black foreground then sat on a dark row and became
   invisible. It struck exactly where the eye goes: the row the selection is on, which under the
   anchor rule is the current match. Predates this sprint (the wash arrived in 015); the wash now
   skips cells that already carry a background, with a `TestBackend` regression test.
2. A **dim row kept `bold_dirs`**, so a non-matching directory rendered bold-and-dim and still read
   as prominent while being inert. Dim rows now drop the bold.
3. **A dim folder's chevron stopped working.** ADR 0023 made dim rows inert, and the click path took
   that literally, so a narrowing froze the tree's shape — a folder that did not match could not be
   opened or closed. A chevron is structure, not selection: it now toggles on a dim row too, without
   moving the selection. Everything else about a dim row stays inert.
4. **`--filter '*/'` matched nothing and `hide` made rows vanish while browsing.** Two causes.
   `globset` matches strings and has no notion of directories, so a directory was offered to it as
   `src` and standard trailing-slash semantics could not apply; directories are now presented as
   `src/`, which makes `*/` mean "any directory" exactly as a shell or `.gitignore` reads it, and a
   trailing slash no longer misclassifies a pattern as a path rule. Separately, `hide` dropped
   directories "known" to hold no match — but the tree loads lazily, so that knowledge arrived
   mid-browse and rows disappeared under the cursor. `hide` now drops files only.
5. **A search snapped a scrolled viewport back.** `rematch` runs on every index refresh, and it
   revealed the current match unconditionally, so any filesystem churn during a search yanked the
   pane back to the selection. It now reveals only when the anchor actually moves. A second, milder
   case was fixed alongside: `sync` re-pointing a selection through a compacted chain counted as
   movement, so bookkeeping could drag the viewport too.

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
- Implementation: `063` (tree-order stepping + forward anchor), `059` (documentation, after tracing
  the offset to the embedded Symbols Nerd Font), `062` (one search model; the flat picker list
  deleted), `060` (the three-case right arrow), `027` (the glob filter, both modes). Two rendering
  bugs found on screen and fixed. Closed out; gates green.
