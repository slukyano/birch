---
type: Task
title: Make the view filter settable at runtime
description: A config default and a `birch ctl set filter` key for the glob filter, which sprint 016 ships as launch flags only.
status: Draft
priority: low
blocked_by:
- 027-add-picker-filter
---

Sprint 016 ships the glob filter (`027`) as **launch flags only** — `--filter <GLOB>` (repeatable)
and `--filter-mode <hide|skip>` — because a filter is normally chosen per invocation, typically by
an adapter spawning a picker.

Once it also narrows a long-lived tree-mode instance (which `027` allows), runtime control becomes
reasonable:

- a **config key** in `~/.config/birch/birch.toml` for a personal default, taking its place in the
  precedence chain `config < flags < ctl set` (ADR 0022);
- a **`birch ctl set filter '*.md'`** key, plus a way to clear it, so a host adapter can narrow a
  running pane. The protocol's `set` is additive, and `SettingKey::Theme` already carries a
  free-form string value, so the shape exists.

Design questions: how a *list* of patterns is expressed in a single `set` value (repeated calls that
append, or one whitespace/comma-separated value — noting that a comma can occur in a glob's brace
expansion); whether `--filter-mode` gets the same treatment; and what clears the filter.
