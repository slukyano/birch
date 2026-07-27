---
type: ADR
title: The render layer is theme-parameterized; theme id in core, theme definition in tui
status: Accepted
sprint: sprint-015
---

# Context

Every visual decision is currently hardcoded: `crates/birch-tui/src/render.rs` holds the palette
(`SELECTION_BG`, `CHEVRON_COLOR`, git status colors, match box), the selection treatment (full-row
background fill), and the row geometry; `crates/birch-tui/src/icons.rs` holds a single glyph+color
map and one folder glyph. "Earn beautiful" wants a curated, cohesive default *and* the ability to
pick a familiar look (VS Code, JetBrains, Xcode) or a stripped-back one (retro, plain).

Two constraints bind the design:

- **The load-bearing crate boundary** (`docs/design.md`, Architecture #2): `birch-core` must build
  without ratatui. Theme *definitions* are ratatui `Color`s and glyphs — they cannot live in core.
- **The real-tree/render split**: theming is a paint-time transform only. It changes how rows look,
  never what the tree *is*, and never the hit-test geometry (mouse zones must stay put).

# Decision

Introduce a **`Theme`** abstraction in `birch-tui` that parameterizes the whole paint layer, and a
pure-data **`ThemeId`** enum in `birch-core` that selects one.

- **`ThemeId`** (core) — a small enum: `Birch` (default), `Vscode`, `Jetbrains`, `Xcode`, `Retro`,
  `Plain`. It is a `Settings` field and a `protocol::SettingKey` variant (so `birch ctl set theme
  <id>` and the config file can carry it). Core stays ratatui-free — it only names the theme.
- **`Theme`** (tui) — a plain struct holding the resolved visual vocabulary, built by
  `Theme::for_id(ThemeId)`. It controls these axes:
  - **guides** — `GuideStyle { None, Indent, Connectors }`, rendered in the existing indent columns
    (2 per level), so `hit_test` geometry is unchanged;
  - **palette** — name/dir foreground, selection background + accent-bar color, chevron, separator,
    ignored, match box, and the per-`FileStatus` git colors;
  - **badges** — `BadgeStyle` (letter `M`/`A`/… vs. symbol) and the directory-rollup glyph;
  - **icons** — an `IconSet` (which glyph+color map) plus `folder_icon: bool` (show a folder glyph,
    or let the chevron stand in its place, VS Code-style) and a monochrome flag;
  - **selection** — `SelectionStyle { FullRow, SoftBarAccent }`.
- **`render.rs` and `icons.rs` take `&Theme`.** The current hardcoded constants become the `Birch`
  theme's values (re-tuned to the flagship design in `054`); `icon_for` becomes a method on the
  active `IconSet`.
- **Themes are built-in (compiled),** not user-authored files. User-supplied theme files are out of
  scope (a future task if demand appears).
- **Emulation themes are descriptive, not affiliated.** `vscode`/`jetbrains`/`xcode` name the look
  they emulate; no editor logos ship, and a short **Trademarks** disclaimer goes in the README
  (nominative fair use). Icon differentiation is "in the spirit of," approximated from the Nerd
  Font families actually available — not pixel-perfect replicas.

Selection flow: `ThemeId` resolves from **config `theme` → `--theme` flag → `ctl set theme`**
(runtime), the same precedence chain as every other setting (ADR 0022).

# Consequences

- Core stays ratatui-free (compiler-enforced) and owns *which* theme; tui owns *what it looks like*.
  This is a **pragmatic compromise, not a clean split**: putting `ThemeId` in core leaks a
  render-layer concern — core gains awareness of the theme catalog's *identities*. Accepted so the
  theme is selectable through the existing `Settings` / protocol / config plumbing without new
  machinery. A future cleanup (task `055`) moves all visual-style knowledge into tui behind a
  dependency-injection seam, leaving core carrying only an opaque theme token.
- `render.rs`/`icons.rs` signatures gain `&Theme`; the app threads the active theme (derived from
  `Settings.theme`) into `draw`. Live `ctl set theme` re-renders with no restart.
- The `icons` **on/off** setting stays the master switch (Nerd-Font fallback / accessibility);
  a theme's `IconSet` only applies when icons are on. `plain` pairs a no-icon default with basic
  ANSI, but `--no-icons` still works under any theme.
- New surface to keep additive: adding a theme is a new `ThemeId` variant + a `Theme` value; the
  socket protocol tolerates unknown ids from newer clients (rejects with an error, never crashes).

# Alternatives considered

- **Per-color config keys** (let users set every color) — rejected: overwhelming and ugly by
  default; curated bundles are the point of "beautiful."
- **External theme files** (load `.toml`/`.json` themes from disk) — deferred; built-in themes
  cover the stated need without a theme-format commitment.
- **Theme id in tui only** (core carries an opaque token behind a DI seam) — the *preferred end
  state*, but it needs an extensible-settings mechanism that does not exist yet. Deferred to task
  `055`; this ADR takes the enum-in-core shortcut to ship the catalog now.
