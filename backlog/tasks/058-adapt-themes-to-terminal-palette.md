---
type: Task
title: Adapt themes to the terminal color scheme
description: Themes assume a black background; make them respect/adapt to the user's terminal palette (light/dark, base16) instead of only hardcoded RGB.
status: Draft
priority: medium
tags:
- future
blocked_by:
- 054-refine-tree-visual-design
---

Sprint 015's themes use hardcoded RGB tuned for a **black terminal background**. On a light or
differently-tinted terminal they may read poorly (low-contrast guides, a selection fill that fights
the real background). Future work: make themes aware of the terminal's actual color scheme so birch
looks right without the user matching birch's assumptions.

Options to weigh in design:
- **Detect background luminance** (OSC 11 query) and pick a light/dark variant per theme.
- **Honor the terminal's ANSI / base16 palette** where a theme wants to inherit, instead of RGB.
- **Ship light + dark variants** of each theme, selectable (and/or auto).

Depends on the theme engine (`054`). Sequence after the catalog (`025`) so there is a full theme set
to adapt.
