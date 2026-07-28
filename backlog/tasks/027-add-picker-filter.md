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

- The unified picker's contract is "`Enter` always picks" (ADR 0016). A filter introduces rows that
  can be selected but not confirmed; decide the feedback for a rejected `Enter` (status message,
  bell, nothing) and record it — it amends that ADR's absolute.
- The filter restricts the corpus; the fuzzy query ranks what is left. Both the tree rows and the
  search index must respect it, or a query will surface filtered-out entries.
- `skip` shares its "greyed and unselectable" treatment with search non-matches and with `062`'s
  direction for picker search — one concept, implemented once.
- Picker mode only; tree mode is unaffected.

Related: `062` (picker search behaviour).
