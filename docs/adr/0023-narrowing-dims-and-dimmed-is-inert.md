---
type: ADR
title: Narrowing dims instead of replacing rows, and a dimmed row is inert
status: Accepted
sprint: sprint-016
---

# Context

Two narrowing mechanisms had grown apart. In tree mode a search query keeps the tree, marks
matches, and dims everything else; `↑`/`↓` jump between matches. In picker mode the same query
takes a different path entirely — `filter_list_active()` replaces the rows with a dense flat list
of hits (`match_rows`), disables `→`/`←`, and drops the tree structure, its ancestors, and its
context. The same keystrokes taught two different mental models, and the picker — the mode where a
file is being *chosen* and context matters most — was the one that discarded the tree
(`062-unify-search-in-pick-mode`).

A third narrowing was about to arrive: a glob filter over what the tree shows and what may be
picked (`027-add-picker-filter`). Without a single rule, birch would have had three narrowing
behaviours and two kinds of dimming.

Dimming also had no defined semantics. A dimmed row was a hint — still selectable, still openable,
still pickable — so "narrowed" and "reachable" were unrelated properties.

# Decision

**One narrowing model.** A narrowing (a search query, a glob filter) never builds its own row set.
It computes **liveness** over the existing tree, and the tree renders as it always does.

1. **Dimmed is inert.** A row that fails an active narrowing renders dimmed and **cannot hold the
   selection**: keyboard navigation steps over it, a click on it does nothing, and `Enter` never
   acts on it. Dimmed rows stay *visible* — the surrounding tree is the context that makes a
   narrowed view readable.
2. **Each narrowing declares which rows it judges, and a judged row that fails is dimmed.**

   | narrowing | judges | selectable | pickable |
   |---|---|---|---|
   | **search** (a typed query) | every row, files and directories alike | a match | a match |
   | **filter** (`--filter` globs) | files only — directories are never judged | file: a match · directory: always | file: a match · directory: a match |

   A directory that fails the search is therefore **not selectable**: under a query, only matches
   can hold the selection. A directory under a glob filter stays selectable whether or not it
   matches, because file-shaped patterns (`*.md`) match no directory at all, and judging
   directories by them would make the tree unnavigable — but such a directory is not *pickable*,
   which is the rule `027` asks for ("folders stay navigable, and are pickable only when they
   match").
3. **Ancestors of matches stay visible, not selectable.** A dimmed ancestor still renders, so a
   match is always shown in its place in the tree. Reaching it is the job of match stepping, which
   expands through collapsed directories.
4. **Under an active search, "the next row" is "the next match".** `↑`/`↓` step matches in tree
   order (`063`). `→` and `←` keep their structural jobs on a *matching* row — expand, split,
   collapse — and otherwise move to the next or previous match, because nothing else is selectable.
5. **The selection anchors forward.** On every rematch the selection moves to the first match at or
   after the current selection in tree order, wrapping when none follows; a selected row that is
   still a match does not move (`063`).
6. **Composition is intersection.** The filter defines the corpus; a search runs *inside* it. A
   query can never surface something the filter excluded.
7. **Presentation.** Search always dims and never hides — tree-mode search keeps today's feel. The
   filter chooses: `skip` dims non-matching files in place (default), `hide` omits them. **Neither
   mode ever hides a directory.** Hiding directories that hold nothing was tried and withdrawn: the
   tree loads lazily, so "this branch is empty" only becomes known while browsing, and rows vanished
   under the cursor as listings arrived. An empty branch costs less than a tree that rearranges
   itself while being read.
8. **Nothing to select means nothing selected.** When no row can hold the selection, the selection
   is empty, no cursor is drawn, and `Enter` reports "no matches" and picks nothing — so a picker
   can never return a path that the active narrowing excluded.
9. **The flat match list is removed.** Picker and tree render the same tree through the same code
   path; `flat_view::match_rows` and `filter_list_active` are deleted.

# Consequences

- **`016-unify-picker`'s "Enter always picks" is amended.** `Enter` still picks *whatever is
  selected*, with no per-kind branching — but the selection can no longer land on a dimmed row, and
  a directory that the filter does not match reports why instead of being picked. The absolute
  survives as "Enter is never contextual"; what changed is which rows can be selected at all.
- **Tree mode changes too.** During a search the tree becomes a list of matches with context around
  it: `→`, `←`, and clicks no longer reach a non-matching row. Backing out to free navigation is
  `Esc`, which already restores the pre-search selection and scroll.
- **A search hides no structure but reaches none either.** Ancestors are on screen for orientation
  only. This is the deliberate consequence of one dimming rule: what is not a match is not a
  target, whether it is a file or the directory that holds one.
- **The picker loses its dense hit list** and gains ancestors, siblings, git badges, and working
  arrows. In a large tree the hits are further apart on screen than they were in the flat list;
  match stepping (`↑`/`↓`), which reveals through collapsed directories, is the answer to that.
- **Dimming is computed per frame** from the match set and the filter, as a paint-time transform
  over the real tree — the real-tree/render boundary is preserved, and nothing about narrowing
  touches the watcher, git, persistence, or the socket.
- **The index stays unfiltered.** The glob filter is applied to search *results*, not to the index
  walk, so changing the filter never requires an index rebuild.

# Alternatives considered

- **Keep the flat list as an option.** Rejected: selecting between two behaviours is a *mode*, and
  modes are on the `docs/design.md` out-of-scope list. It would also need a hotkey, and printable
  characters are permanently reserved for search.
- **Dim as a pure hint (today's tree-mode behaviour) everywhere.** Rejected: it leaves the picker
  able to confirm a path the filter was created to exclude, which makes `--filter` advisory.
- **Keep ancestors of matches selectable under a search.** Rejected by maintainer decision: a
  directory that does not satisfy the query is not a target, and one rule ("what is dimmed is
  inert") beats an exception for directories.
- **Hide non-matches instead of dimming them.** Rejected as the default: the surrounding context is
  most of the value of narrowing inside a tree. It remains available as the filter's `hide` mode.
