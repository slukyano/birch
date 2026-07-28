---
type: Task
title: Picker mode changes what search does
description: In pick mode a query replaces the tree with a flat match list and disables →/←; tree mode keeps the tree and jumps between matches. The two should feel the same.
status: Draft
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
(ADR 0016) when the selection is a dimmed non-match?

Related: `063` (match cycling order), `027` (picker filter).
