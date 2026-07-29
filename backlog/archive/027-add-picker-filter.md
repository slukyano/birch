---
type: Task
title: Add a picker file filter
description: Glob pattern(s) restricting what the picker can pick, with hide or skip presentation; folders stay navigable but are only pickable when they match.
status: Done
priority: medium
blocked_by:
- 016-unify-picker
---

Maintainer request: picker mode gains a filename filter (macOS Finder-style) — glob patterns
restricting which entries are pickable, plus a presentation mode.

## Surface

- **Patterns**: one or more globs. Preferred spelling is a **repeatable flag** —
  `--filter '*.md' --filter '*.txt'` — unambiguous (no separator can collide with a filename),
  standard for CLIs, and a plain `Vec<String>` in clap. If the glob crate supports brace expansion,
  `--filter '*.{md,txt}'` then works as a shorthand for free; a comma-separated list is the weakest
  option and should not be the only spelling. An entry matches if it matches **any** pattern.
- **Mode**: `--filter-mode hide|skip`.
  - `hide` — non-matching files are not shown at all;
  - `skip` — non-matching files are shown greyed out and cannot be selected, exactly like
    non-matches during a search.

  `skip` is proposed as the default (keeps surrounding context visible and reuses the search
  dimming); design should confirm.
- **Folders**: always shown and always selectable, so the tree stays navigable — but a folder can
  only be **picked** (confirmed with `Enter`) when it matches the patterns.

## Notes

- The unified picker's contract is "`Enter` always picks" (the `016-unify-picker` rule; ADR 0016 is
  the unrelated cmux Dock decision). A filter introduces rows that
  can be selected but not confirmed; decide the feedback for a rejected `Enter` (status message,
  bell, nothing) and record it — it amends that ADR's absolute.
- The filter restricts the corpus; the fuzzy query ranks what is left. Both the tree rows and the
  search index must respect it, or a query will surface filtered-out entries.
- `skip` shares its "greyed and unselectable" treatment with search non-matches and with `062`'s
  direction for picker search — one concept, implemented once.
- Picker mode only; tree mode is unaffected.

Related: `062` (picker search behaviour).

## Design

Backed by **[ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)** — narrowing dims
instead of replacing rows, and a dimmed row is inert. `062` implements that decision for search;
this task implements it for the glob filter, and the two share one liveness computation.

**The filter is not picker-only.** Maintainer direction during design: the same rules apply in tree
mode, where the filter is simply a view narrowing (dim or hide non-matching files) with no pick
semantics attached. The task's original "picker mode only; tree mode is unaffected" note is
superseded.

**Surface.**

| flag | shape | meaning |
|---|---|---|
| `--filter <GLOB>` | repeatable, `Vec<String>` | an entry matches when it matches **any** pattern. Valid in both modes. |
| `--filter-mode <hide\|skip>` | enum, default `skip` | `skip` dims non-matching rows in place; `hide` omits them. Requires `--filter`. |

**Matching corpus mirrors search (ADR 0013).** A pattern **without** `/` matches the entry's
**name**; a pattern **containing** `/` matches the **root-relative path**. So `*.md` is a filename
rule and `src/**/*.rs` is a path rule, with no extra flag to explain the difference.

**Globbing via `globset`** — already in the dependency tree under `ignore`, so brace expansion
(`*.{md,txt}`) and `**` come free and no new transitive crate is added. Patterns compile once at
startup into a `GlobSet`; a malformed pattern is a startup error naming the pattern, before the TUI
takes the terminal.

**What the filter judges.** Only **files**. A **directory** is never judged by the filter, so it
never dims and stays selectable and navigable — file-shaped patterns such as `*.md` match no
directory at all, and judging directories by them would make the tree unnavigable. A directory is
**pickable** only when it matches the patterns itself. This is the task's folders rule, and ADR 0023
records it as the filter's declared policy (the search, by contrast, judges every row).

**`hide` hides files only** (revised after live use). The design first dropped directories with no
live descendant as well, on the reasoning that empty branches are noise. In practice the tree loads
lazily, so "this branch holds nothing" became known *while browsing* and rows disappeared under the
cursor — far worse than a branch that turns out empty. Directories therefore always stay.

**Directory patterns come from glob itself.** `globset` matches strings and knows nothing of file
kinds, so a directory is presented to it with a trailing `/`. Standard semantics then apply with no
special case: `*/` names any directory, `*.md` names no directory. A *trailing* slash also does not
make a pattern a path rule — `*/` is a name rule ("any directory, at any depth"), `src/*/` a path
rule — which is how a `.gitignore` reads them.

**Search composes by intersection.** The filter defines the corpus; a query runs inside it. The
index itself stays unfiltered — filtering the *results* keeps a runtime filter change from forcing
an index rebuild — so a query can never surface a filtered-out entry.

**`Enter` on an unpickable row** emits a status message naming the reason (`does not match the
filter`), reusing the existing `NavEffect::Message` channel that already reports
`X is deleted (tracked in git)`. Nothing is picked and the picker stays open.

**Public surface.**

- **CLI**: `--filter <GLOB>` (repeatable) and `--filter-mode <hide|skip>` (default `skip`), both
  valid with and without `--pick`.
- **Config**: **none, by decision** — a filter is chosen for a task, not kept as a personal default;
  a default filter would silently hide files in every future session.
- **Protocol / `ctl set`**: **none, by decision** — narrowing a pane that a host adapter shares is
  not a use that exists, and a repeatable glob list has no natural single-value spelling.
- **Environment variables, on-disk paths, public APIs**: unchanged.

**Tests.**

- Name patterns vs path patterns: `*.md` matches by name at any depth; `src/*.rs` matches by
  relative path only.
- Multiple `--filter` flags union; a malformed glob fails at startup with the pattern named.
- `skip`: non-matching files render dimmed, cannot be selected by keyboard or mouse, and `Enter`
  on one reports instead of picking.
- `hide`: non-matching files and dead-end directories are absent from the rows; directories holding
  matches remain.
- A directory that does not match the patterns is selectable and navigable but not pickable.
- Filter ∩ search: a query cannot surface a filtered-out entry.
- Tree mode honours the same filter (no pick semantics involved).
