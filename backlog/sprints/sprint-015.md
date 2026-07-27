---
type: Sprint
title: Visual design — earn "beautiful"
status: Implementing
branch: sprint/015
tasks:
- 054-refine-tree-visual-design
- 025-add-visual-styles
- 031-add-config-file
- 052-fix-reveal-symlink-canonicalization
- 057-remove-files-first
---

# Scope rationale

The publication arc is closed and birch looks the part on the outside (logo, tagline, README). This
sprint makes the **tree itself** beautiful — and does it as a **theme system** rather than a
one-off restyle. Every render decision (guides, palette, badges, icons, folder-icon, selection)
becomes an axis; a *theme* is a coherent point in that space. A **config file** persists the chosen
theme (and the existing toggles), because a theme you can't keep isn't finished. One high-priority
DX bug (`052`) rides along.

The theme *definitions* are render-layer (ratatui colors/glyphs) and live in `birch-tui`; the theme
*id* is pure data in `birch-core` — the load-bearing crate boundary (core builds without ratatui)
is respected. No new features, verbs, modes, or hotkeys; printable characters stay reserved for
search; the real-tree/render split holds.

# In-scope task ledger

- **`054-refine-tree-visual-design`** — *major, design-heavy.* The **theme engine** plus the
  flagship **`birch`** theme (the beautiful default): a `Theme` abstraction parameterizing
  `render.rs`/`icons.rs` over guides, palette, badges, icon set, folder-icon, and selection; a
  pure-data `ThemeId` in `birch-core`. Backed by the theme-system ADR.
- **`025-add-visual-styles`** — *major, design-heavy.* The **theme catalog** built on `054`'s
  engine: `vscode`, `jetbrains`, `xcode`, `retro`, `plain`. `--theme <id>` flag and a `ctl set
  theme` key. Trademark disclaimer for the emulation-named themes.
- **`031-add-config-file`** — *mid, design-heavy.* `~/.config/birch/birch.toml` (XDG) sets the
  default `theme` and the existing `Settings` toggles. Precedence **config < CLI flags < `ctl
  set`**. Tolerant TOML parsing in `birch-core`. Backed by a **separate** config ADR.
- **`052-fix-reveal-symlink-canonicalization`** — *minor (bug), high.* Match the reveal path
  as-given, resolving symlinks only as a fallback, so `birch ctl reveal /tmp/...` isn't wrongly
  "outside the root" on macOS. Independent of the visual work.
- **`057-remove-files-first`** — *minor, low.* Drop the `files-first` sort toggle (setting, flag,
  and protocol key); directories always sort first. Folded in because it edits the same
  `Settings` / `SettingKey` / CLI surface as the theme and config work.

# Ordering / dependencies

- `054` lands the **engine + `birch` theme first**; `025` populates the catalog on top of it (the
  catalog cannot precede the engine).
- `031` can be designed in parallel but implements against the `ThemeId` and `Settings` that `054`
  defines; the precedence chain (default → config → flags → `ctl set`) is its backbone.
- `052` is independent — any point.

# Considered but out of scope

- **User-authored theme files** — themes are built-in (compiled) this sprint; loading themes from
  disk is a future task if demand appears.
- **Per-key colour config** — the config sets a *theme* + toggles, not individual colours; curated
  bundles beat a hundred colour keys.
- **`029`/`034`** (context menu, open-with), **`053`** (state-persistence toggle),
  **`026`/`027`/`028`/`030`/`032`/`033`/`035`/`051`** — feature/integration work, off-theme.

# Sprint-start action

Scope committed to `main`; branch `sprint/015` cut. Design phase in progress (this expands the
original three-task scope with `031` per maintainer direction).

# Checklist

- [ ] 054-refine-tree-visual-design
- [ ] 025-add-visual-styles
- [ ] 031-add-config-file
- [ ] 052-fix-reveal-symlink-canonicalization
- [x] 057-remove-files-first

# Open questions

_(none blocking — design decisions captured in the task `## Design` sections and ADRs 0021/0022,
pending design approval.)_

# Session log

- Scoped and cut: `054`, `025`, `052`. Branch `sprint/015` cut from `main`.
- Design phase: reframed `054`/`025` as a **theme system** (engine + flagship `birch` theme, then a
  catalog: `vscode`/`jetbrains`/`xcode`/`retro`/`plain`) and pulled in `031` (config file) to
  persist the chosen theme. Two Proposed ADRs — 0021 (theme system) and 0022 (config file). Per-task
  designs written for `054`/`025`/`031`/`052`.
- Design review round: seeded `055` (tech debt — encapsulate visual styles in tui; ADR 0021 now owns
  the abstraction leak) and `056` (future — user themes); folded in `057` (drop `files-first`);
  switched config overrides to **bidirectional flags** (ADR 0022); refined `052` to match the path
  as-given and canonicalize only as a fallback; config bad-input warnings go to stderr, never the
  TUI; dropped `--print-config-path`, added `--config <path>`.
