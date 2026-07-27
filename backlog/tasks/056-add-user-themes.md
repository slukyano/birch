---
type: Task
title: Support user-authored themes
description: Load custom themes from disk (e.g. ~/.config/birch/themes/*.toml) so users can define their own visual styles, not just the built-in catalog.
status: Draft
priority: low
tags:
- future
blocked_by:
- 054-refine-tree-visual-design
---

Sprint 015 (ADR 0021) ships **built-in** themes only. The natural follow-on: let users author their
own. Load theme definitions from disk (e.g. `~/.config/birch/themes/<name>.toml`) covering the same
axes the built-in themes use — guides, palette, badges, icon set, folder-icon, selection —
selectable by name via `--theme`, `birch ctl set theme`, and the config `theme` key, alongside the
built-ins.

Needs a **stable, documented theme file format** (a public contract, additive-only once shipped) and
tolerant parsing. Depends on the theme engine (`054`) and reads best after the tui-encapsulation
cleanup (`055`), which gives the render layer full ownership of the theme vocabulary. Future / low
priority — gauge demand before committing to a file format.
