---
type: Task
title: Add visual styles
description: Selectable styles - default (Nerd Font), vscode (no folder icons, compact), plain (no icons, compact).
status: Designed
priority: medium
---

Maintainer request: a style setting selecting coherent visual presets — `default`
(current: Nerd Font icons, chevrons), `vscode` (no folder icons, tighter indentation),
`plain` (no icons at all, most compact — also the no-Nerd-Font fallback). Interacts
with `--no-icons` (which becomes a style alias or is absorbed); a `birch-ctl set`
key and, later, the config file select it. Note: sprint-010 removed the open-folder
glyph flip (the chevron alone carries expansion state); an open-folder variant could
return here as part of a style rather than the default.

Scope: this task is only about **choosable presets** (the style setting and what each preset
toggles). The general visual design of the tree — indent/tree guides, palette, selection styling,
spacing — lives in [`054-refine-tree-visual-design`](054-refine-tree-visual-design.md); presets
build on that baseline (e.g. the classic-connector guide look could be one preset).

## Design

Reframed as the **theme catalog** on top of `054`'s engine (ADR 0021). Each theme is a `Theme` value
selecting a coherent point across guides / palette / badges / icon-set / folder-icon / selection:

| `ThemeId` | guides | folder icon | icon set | selection / palette |
|-----------|--------|-------------|----------|---------------------|
| `birch` *(default, in 054)* | Indent | shown | curated Nerd Font, muted | soft bg + green accent bar |
| `vscode` | Indent | none (chevron in its place) | Seti/codicon-ish Nerd Font | VS Code blues, bar accent |
| `jetbrains` | None | shown | IDEA-ish (warm, near-mono) | warm-gray soft selection |
| `xcode` | None | shown | colored file-type icons | macOS-blue full-row selection |
| `retro` | Connectors | ASCII/legacy glyphs | blocky Nerd Font | high-contrast ANSI |
| `plain` | Connectors | none | none (icons off) | basic 16-color ANSI |

**Selection surface.** `--theme <id>` (clap `ValueEnum`); `SettingKey::Theme` with `birch ctl set
theme <id>` (the `set` verb's value is already a free `String` — parse it to `ThemeId`, error on an
unknown id). Config carries the persisted default (`031`).

**Icons.** `IconSet` is the glyph+color map behind `icon_for`. `birch`/`vscode`/`jetbrains` share
the curated Nerd Font glyphs with per-theme color treatment; `retro` uses a blocky/legacy set;
`xcode` a colored file-type set; `plain` none. Differentiation is *in the spirit of* each editor —
approximated from the Nerd Font families that exist, not pixel-perfect. `retro`/`plain` are trivial.

**Master switches stay.** `--no-icons` / `icons=false` overrides any theme's icon set (Nerd-Font
fallback); `plain` merely defaults icons off. `--no-git` still hides badges under any theme.

**Trademarks.** `vscode`/`jetbrains`/`xcode` describe the look they emulate; ship no editor logos
and add a short **Trademarks** disclaimer to the README (nominative fair use; birch unaffiliated).

**Public surface.**
- CLI: **`--theme <birch|vscode|jetbrains|xcode|retro|plain>`** (clap `ValueEnum`).
- ctl: **`birch ctl set theme <id>`** — `SettingArg` gains a `Theme` variant; the `set` value is
  parsed as a `ThemeId`, an unknown id errors cleanly. (No new socket *verb* — reuses `Set`.)
- README: a short **Trademarks** section (bottom) for the emulation-named themes.
- No env vars, no new on-disk paths (the persisted default lives in the config file, `031`).

**Implementation deliverables (maintainer checkpoints).**
- **Show all themes, compared, before the task is called done** — a side-by-side visual (screenshots
  of the same tree under each theme) presented to the maintainer for sign-off; the aesthetic is
  the deliverable, so this is a stop-and-show, not a self-approval.
- **Document the themes in the README** — a Themes section listing each id and its look, ideally a
  **compact grid of screenshots + names**. (Screenshot assets under `docs/assets/`.)

**Tests:** every `ThemeId` resolves to a `Theme`; `--theme` and `ctl set theme` select it; an
unknown id errors cleanly.
