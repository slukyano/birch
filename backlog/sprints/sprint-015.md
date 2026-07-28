---
type: Sprint
title: Visual design — earn "beautiful"
status: Done
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

- [x] 054-refine-tree-visual-design
- [x] 025-add-visual-styles
- [x] 031-add-config-file
- [x] 052-fix-reveal-symlink-canonicalization
- [x] 057-remove-files-first

# Open questions

_(none — all resolved in chat during design and the design-review rounds.)_

# Sprint summary

The tree is themed. A `Theme` abstraction in `birch-tui` (ADR 0021) parameterizes every paint
decision — palette, indent-guide style (uniform / depth-fading / classic connectors), selection
treatment (full-row / soft wash + accent bar, with optional fg swap), chevron glyphs, folder
layout (`Icon`/`Compact`/`Plain`), badge glyph, and per-theme Nerd Font icon families with a
tint override — selected by a pure-data `ThemeId` in `birch-core` (the crate stays ratatui-free).

Eleven built-in themes, produced through a research-driven workshop (editor ground truth captured
from the real apps on-screen and pixel-sampled; glyph codepoints verified from tool sources; TUI
design synthesis; an eight-app survey) and two rounds of an adversarial design review:

- **`birch`** (default) — "silver bark with a single gold stroke": desaturated silver-green tree,
  bold bark-silver dirs, one sage tint for all icons, depth-fading indent guides, and a gold
  accent bar over an edge-to-edge warm wash as the only saturated mark.
- **`vscode` / `jetbrains` / `xcode`** — measured mimics (layouts, thin chevrons, icon families,
  selection colors from the real apps).
- **`mocha` / `tokyonight` / `gruvbox` / `nord` / `rosepine`** — official-palette scheme themes
  with theme-owned icon hues.
- **`retro`** — the Commander: canonical CGA `#0000AA` canvas, black-on-cyan cursor bar, white
  bold dirs, `+`/`-` marks, CP437 bullets, no icons.
- **`plain`** — ANSI-safe, icon-free fallback.

The config file (ADR 0022) landed at `~/.config/birch/birch.toml` with tolerant TOML parsing in
core, `--config <path>`, and bidirectional CLI toggles completing the precedence chain
`default → config → flags → ctl set`. `reveal` now resolves symlinked prefixes (as-given first,
canonicalize as fallback; `052`), and the `files-first` setting is gone (`057`). The catalog rule
"semantics global, hues local" is enforced in the engine (`icon_tint`, themed badge glyph — no
global constant punches through a palette). Research preserved in `docs/research/`; the workshop's
every variant archived in a grouped PDF (session artifact). Released as **v0.1.1**.

Deliberately not done: guide-ancestry brightening and the header-path ellipsis (cosmetic
polish), per-theme distinct glyph *sets* beyond the four families (`TODO(025)` note retained) —
all small; terminal-palette adaptation is `058`, tui-encapsulation tech debt is `055`, user
themes are `056`.

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
- Implementation: `057` and `052` first (mechanical), then the theme engine (`054`), config
  (`031`), and the catalog (`025`) — iterated through research (real-app captures + glyph-source
  verification + TUI-design and app surveys, seeded `058`) and two adversarial design-review
  rounds; the flagship rebuilt as "silver bark with a single gold stroke", retro rebuilt as the
  Commander, the hues-local rule enforced in the engine. Docs: README Themes/Configuration
  sections, `docs/research/` bundle. Closed out and released as v0.1.1.
