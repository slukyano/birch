---
type: Task
title: Add a picker file filter
description: Glob pattern(s) restricting what the picker can pick, with hide or skip presentation; folders stay navigable but are only pickable when they match.
status: Draft
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

**What the filter classifies.** Only **files** are matched against the patterns. A **directory** is
live when it holds a live descendant, per ADR 0023 — so directories stay navigable — and is
**pickable** only when it matches the patterns itself. That is the task's folders rule, expressed by
the shared liveness/pickability split rather than a filter-specific branch.

**`hide` hides dead ends too.** In `hide` mode a non-matching *file* is omitted, and so is a
directory with no live descendant: keeping empty branches would fill the pane with paths that lead
nowhere. Directories that do hold matches stay, which is what "folders stay navigable" is for.

**Search composes by intersection.** The filter defines the corpus; a query runs inside it. The
index itself stays unfiltered — filtering the *results* keeps a runtime filter change from forcing
an index rebuild — so a query can never surface a filtered-out entry.

**`Enter` on an unpickable row** emits a status message naming the reason (`does not match the
filter`), reusing the existing `NavEffect::Message` channel that already reports
`X is deleted (tracked in git)`. Nothing is picked and the picker stays open.

**Public surface.**

- **CLI**: `--filter <GLOB>` (repeatable) and `--filter-mode <hide|skip>` (default `skip`), both
  valid with and without `--pick`.
- **Config**: none this sprint — the filter is a per-invocation narrowing, not a personal default.
- **Protocol / `ctl set`**: none this sprint (see `067-runtime-filter-control` if runtime control is
  wanted later).
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
