---
type: Task
title: Add visual styles
description: Selectable styles - default (Nerd Font), vscode (no folder icons, compact), plain (no icons, compact).
status: Draft
priority: medium
---

Maintainer request: a style setting selecting coherent visual presets — `default`
(current: Nerd Font icons, chevrons), `vscode` (no folder icons, tighter indentation),
`plain` (no icons at all, most compact — also the no-Nerd-Font fallback). Interacts
with `--no-icons` (which becomes a style alias or is absorbed); a `birch-ctl set`
key and, later, the config file select it. Note: sprint-010 removed the open-folder
glyph flip (the chevron alone carries expansion state); an open-folder variant could
return here as part of a style rather than the default.
