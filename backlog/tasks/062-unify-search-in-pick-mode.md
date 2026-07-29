---
type: Task
title: Picker mode changes what search does
description: In pick mode a query replaces the tree with a flat match list and disables →/←; tree mode keeps the tree and jumps between matches. The two should feel the same.
status: Designed
priority: high
---

Maintainer report: `--pick` changes the behaviour of search, in a bad way.

## Current behaviour

Two different interaction models behind the same keystrokes (`crates/birch/src/app.rs`):

| | tree mode | picker mode |
|---|---|---|
| what a query does | the tree stays; matches are highlighted and `↑`/`↓` jump between them | `filter_list_active()` — matches **replace** the rows; the tree structure disappears |
| `→` / `←` | expand / collapse / parent | **disabled** while the query is non-empty |
| context | ancestors, siblings, git badges stay on screen | flat list of hits only |

So the same tool teaches two different mental models depending on the flag it was launched with,
and the picker — the mode where you are *choosing* a file and most want context — is the one that
throws the tree away.

## Direction

Make picker search behave like tree search: keep the tree, dim non-matches, keep `→`/`←` working.
This also lines up with the picker filter (`027`), whose `skip` mode greys out non-matching files
in place rather than hiding them — one dimming concept across search and filter instead of two.

Open questions for design: does the flat list survive as an option (some pickers want it), and if
so is it a mode, a key, or gone entirely? What happens to the `Enter`-always-picks contract
(the `016-unify-picker` rule — *not* ADR 0016, which is the cmux Dock decision) when the selection
is a dimmed non-match?

Related: `063` (match cycling order), `027` (picker filter).

## Design

Backed by **[ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)** — narrowing
dims instead of replacing rows, and a dimmed row is inert. This task implements that decision for
search; `027` implements it for the glob filter.

**The flat list is deleted.** `filter_list_active()` and `flat_view::match_rows` go away, with the
four behaviours they gated:

| today | after |
|---|---|
| `rows()` returns `match_rows` in picker mode with a query (`app.rs:903`) | always `visible_rows`, with `matched` decor in **both** modes (drop the `mode == Tree` test at `app.rs:907`) |
| `→` is a no-op, `←` is skipped (`app.rs:343`, `app.rs:350`) | both navigate, in both modes |
| a chevron click counts as a name click (`app.rs:641`), and a dir double-click picks rather than browses (`app.rs:668`) | tree semantics everywhere: chevrons toggle, dir double-clicks browse |
| `rematch` snaps the selection to the top match and scroll to 0 (`app.rs:796`) | `reveal`s the current match, exactly as tree mode does (`app.rs:785`) |

`Esc` also gains the picker-mode restore that only tree mode had (`app.rs:743`): clearing a search
returns the pre-search selection and scroll.

**Selectability replaces the hit flag.** `Row.search: Option<bool>` today means "a search is active,
and this row is/isn't a hit". Under ADR 0023 the search judges **every** row, files and directories
alike, so the flag becomes `Row.live: bool` with a simpler definition than a subtree test: a row is
live iff it is a match. Ancestors of matches still render — dimmed — so a match is always shown in
its place in the tree. With no narrowing active every row is live and nothing dims.

`match_indices` stays as it is: the lit characters are about *display*, not reachability. The
renderer dims `!live`; `hit_test` returns the row exactly as before, and the *app* refuses to select
it — so mouse geometry stays untouched.

**Selection can only rest on a live row.** `FlatView` gains this awareness in one place: the
`select`/`sync`/`move_by` path skips non-live rows, and `sync` re-homes a selection that just went
dim. With zero live rows the selection is `None`, the cursor is not drawn, and `Enter` reports
`no matches`.

**Under a search, `→`/`←` fall back to match stepping.** They keep their structural jobs on a
matching row — `→` expands or splits, `←` collapses — but a non-matching parent or sibling cannot be
selected, so otherwise they move to the next/previous match. `↑`/`↓` step matches in tree order in
**both** modes now (the `mode == Tree` guard at `app.rs:325` goes). `Esc` is how free navigation
comes back.

**Bottom line.** The picker keeps its `>` prompt and gains the match counter:
`> {query} ({i}/{n})`, and `> {query} (no matches)` when nothing matches — so a picker session says
the same thing tree mode says, in the picker's voice.

**Public surface.** None. No new flags, config keys, protocol fields, environment variables, or
on-disk paths. `--pick`'s contract is unchanged in the only way a caller can observe it: stdout
carries the picked path, stderr the UI. The *interaction* changes (documented in the README picker
section and `docs/design.md`), and `Enter` on a live-but-non-matching directory now reports instead
of picking — see ADR 0023's amendment to `016-unify-picker`.

**Tests.**

- Picker mode with a query renders tree rows (ancestors present, depth preserved), not a flat list.
- `→` on a matching collapsed directory expands it in picker mode; a chevron click toggles rather
  than picks.
- Navigation skips dimmed rows in both modes; a click on a dimmed row leaves the selection alone.
- A non-matching ancestor directory is visible and dimmed, and cannot be selected or picked.
- A selection that goes dim on the next keystroke re-homes to the next match (`063`'s anchor rule).
- Zero matches: no selection, `Enter` picks nothing and reports `no matches`.
- `Esc` restores the pre-search selection and scroll in picker mode.
