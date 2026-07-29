---
type: ADR
title: Narrowing dims instead of replacing rows, and a dimmed row is inert
status: Proposed
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

1. **Liveness.** A node is **live** when it matches every active narrowing itself, **or** it is a
   directory with a live descendant. Ancestors of matches stay live, so the tree remains navigable
   down to any match.
2. **Dimmed is inert.** Non-live rows render dimmed and **cannot hold the selection**: keyboard
   navigation steps over them, a click on one does nothing, and `Enter` never acts on one.
3. **Two reachable states.** *Live* rows are selectable. **Pickable** is stricter — a row is
   pickable only when it matches every active narrowing *itself*. A directory that is live only
   because something under it matches is therefore navigable but not pickable, which is exactly the
   rule `027` asks for ("folders stay navigable, and are pickable only when they match"). For files
   the two coincide, because a file is live only by matching.
4. **Match stepping stays distinct from navigation.** `↑`/`↓` under an active search step the
   **matches** in tree order (`063`); `→`/`←` and the mouse move over **live** rows. So an ancestor
   directory is reachable by navigating, and skipped when stepping matches.
5. **Composition is intersection.** The filter defines the corpus; a search runs *inside* it. A
   query can never surface something the filter excluded.
6. **Presentation.** Search always dims and never hides — tree-mode search keeps today's feel. The
   filter chooses: `skip` dims non-live rows in place (default), `hide` omits them, including
   directories with no live descendant, which are dead ends under that filter.
7. **Nothing live means nothing selected.** When no row is live, the selection is empty, no cursor
   is drawn, and `Enter` reports "no matches" and picks nothing — so a picker can never return a
   path that the active narrowing excluded.
8. **The flat match list is removed.** Picker and tree render the same tree through the same code
   path; `flat_view::match_rows` and `filter_list_active` are deleted.

# Consequences

- **`016-unify-picker`'s "Enter always picks" is amended.** `Enter` still picks *whatever is
  selected*, with no per-kind branching — but the selection can no longer land on a dimmed row, and
  a live-but-non-matching directory reports why instead of being picked. The absolute survives as
  "Enter is never contextual"; what changed is which rows can be selected at all.
- **Tree mode changes too.** During a search, `→`/`←` and clicks now skip dimmed rows. This is the
  price of one rule instead of two, and it makes an active search feel like a narrowed tree rather
  than a highlighted one.
- **The picker loses its dense hit list** and gains ancestors, siblings, git badges, and working
  arrows. In a large tree the hits are further apart on screen than they were in the flat list;
  match stepping (`↑`/`↓`), which reveals through collapsed directories, is the answer to that.
- **Liveness is computed per frame** from the match set and the filter, as a paint-time transform
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
- **Hide non-matches instead of dimming them.** Rejected as the default: the surrounding context is
  most of the value of narrowing inside a tree. It remains available as the filter's `hide` mode.
