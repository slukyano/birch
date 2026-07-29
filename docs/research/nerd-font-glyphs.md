# Nerd Font glyph reference for tree rendering

Codepoints verified against Nerd Fonts v3.4.0 `glyphnames.json` and the tools' own source files.
Used by the theme catalog (`crates/birch-tui/src/theme.rs`, `icons.rs`).

## Expand/collapse chevrons — what tools actually use

| Tool | Collapsed | Expanded | Source |
|---|---|---|---|
| VS Code explorer | U+EAB6 (codicon chevron-right) | U+EAB4 (chevron-down; collapsed is the same glyph CSS-rotated) | `vscode/src/vs/base/common/codicons.ts` |
| nvim-tree.lua (default) | U+F460 (octicon chevron-right) | U+F47C (octicon chevron-down) | `lua/nvim-tree/config.lua` |
| neo-tree.nvim (default) | U+F460 | U+F47C | `lua/neo-tree/defaults.lua` |
| lazygit | U+25B6 `▶` | U+25BC `▼` | `pkg/gui/presentation/files.go` |
| NERDTree | U+25B8 `▸` | U+25BE `▾` | `doc/NERDTree.txt` |
| eza / lsd / broot | connectors only: U+251C `├`, U+2500 `─`, U+2502 `│`, U+2514 `└` | | each tool's tree renderer |

Plain-Unicode thin pair (no Nerd Font): `›` U+203A + `⌄` U+2304 — both East-Asian-Width
Neutral (single-width safe); U+2304 has spotty coverage in older monospace fonts.

**Width hazards** (East Asian Width = Ambiguous — render double-width on CJK-configured
terminals; avoid in defaults): `▶` U+25B6, `▼` U+25BC, `▷`, `▽`, `∨` U+2228. Width-safe
triangles: `▸` U+25B8 / `▾` U+25BE. Always-wide (never use): `〉` U+232A/U+3009.

Modern macOS (Big Sur+) sidebars use thin SF-Symbol chevrons; the filled disclosure triangle
is the pre-2020 look ([Six Colors Big Sur review](https://sixcolors.com/post/2020/11/macos-big-sur-review-third-age-of-mac/)).

## The fallback-font hazard: icons that are not one cell wide

A terminal whose **primary** font carries no private-use-area codepoints substitutes a
symbols-only font for every icon and chevron. That fallback is frequently the **non-`Mono`**
Nerd Font build, whose glyphs are 1.1–1.7 cells wide and are not centred in the cell — so the
chevron drifts right of the indent guide below it, which is drawn from the primary font and *is*
centred. Nothing in the renderer can correct this: a terminal addresses whole cells, and the
substitution happens below that level.

Measured with `fontTools` (advances normalised to a 0.6 em cell, the JetBrains Mono advance):

| glyph | Symbols Nerd Font (fallback) | JetBrainsMono NFM (`Mono`) | JetBrainsMono NF (non-`Mono`) |
|---|---|---|---|
| octicon chevron U+F460 | adv 1.67 cells, ink centre **+0.42 cell** | adv 1.00, ink centre +0.06 | adv 1.00, ink centre **+0.42** |
| codicon chevron U+EAB6 | adv 1.67 cells, ink centre **+0.37 cell** | adv 1.00, ink centre +0.04 | adv 1.00, ink centre +0.06 |
| folder U+E5FF | adv 1.46, ink **1.46 cells** | adv 1.00, ink 1.00 | adv 1.00, ink **1.46 cells** |
| guide U+2502 | absent (primary font draws it) | ink centre +0.00 | ink centre +0.00 |

Two separate defects hide behind one symptom, and only the second is what "`Mono` variant" fixes:

- **advance width** — the fallback advances 1.1–1.7 cells, so the terminal must squeeze or
  double-width the glyph;
- **ink placement** — even at a 1.00-cell advance, a non-`Mono` build draws ink outside the cell
  and off its centre. Both installed JetBrainsMono Nerd Font variants advance exactly one cell;
  only the `Mono` one also keeps the ink inside it.

**Practical rule:** the icons need a `Mono` Nerd Font as the terminal's **primary** family. Naming
a Nerd Font that the terminal then only reaches by fallback is not equivalent. Family names are
abbreviated in the Nerd Fonts releases — `JetBrainsMono NFM` (Mono), `JetBrainsMono NF`,
`JetBrainsMono NFP` (proportional) — and a misspelled family fails silently back to the same
substitution.

**Known upstream** in Ghostty, whose bundled fallback is "Symbols Nerd Font" (not the `Mono`
build), alongside an unpatched JetBrains Mono as the default primary:

- [discussion 8822](https://github.com/ghostty-org/ghostty/discussions/8822) — "Nerd fonts glyph
  width in 1.2.0": patched glyphs began occupying two cells and stopped aligning with regular
  characters. Answered with the icon scaling/alignment work in PRs
  [8563](https://github.com/ghostty-org/ghostty/pull/8563),
  [8847](https://github.com/ghostty-org/ghostty/pull/8847) and, for macOS,
  [8580](https://github.com/ghostty-org/ghostty/pull/8580).
- [issue 9076](https://github.com/ghostty-org/ghostty/issues/9076) — "Incorrect scale groups
  because codepoint offset is assumed constant per patchset", which names **the Octicon chevrons**
  as a group that is scaled and aligned incorrectly. Closed 2025-10-11.
- [discussion 7204](https://github.com/ghostty-org/ghostty/discussions/7204) and
  [13298](https://github.com/ghostty-org/ghostty/discussions/13298) — the same shape of problem
  from the other direction: a fallback font's metrics differ from the primary's, so substituted
  glyphs are inconsistent with the text around them.

Ghostty asks for a discussion before an issue
([3558](https://github.com/ghostty-org/ghostty/issues/3558)), so any further report starts there.

## Category glyphs per family (as used by `IconSet`)

| category | Devicons | Codicons | Material | Octicons |
|---|---|---|---|---|
| folder | U+E5FF | U+EA83 | U+F0256 (outline) | U+F413 |
| folder (open variants) | U+E5FE | U+EAF7 | U+F0770 / U+F0DCF (outline) | U+F4D4 (fill) |
| generic file | U+F016 | U+EA7B | U+F0224 (outline) | U+F4A5 |
| markdown | U+E73E | U+EB1D | U+F0354 | U+F48A |
| json | U+E60B | U+EB0F | U+F0626 | — |
| config / gear | U+E615 | U+EAF8 | U+F0493 | U+F423 |
| image | U+F1C5 | U+EAEA | U+F02E9 | U+F4E5 |
| license / law | U+F24E | U+EB12 | U+F05D1 | U+F495 |
| lock | U+F023 | U+EA75 | U+F033E | U+F456 |

Material (`nf-md-*`) codepoints are supplementary-plane (six hex digits). Additional folder
options: fa filled U+F07B/U+F07C, fa outline U+F114/U+F115, seti U+E613, octicon fill U+F4D3.

Sources: [nerd-fonts glyphnames.json](https://github.com/ryanoasis/nerd-fonts/blob/master/glyphnames.json),
[vscode-codicons mapping.json](https://github.com/microsoft/vscode-codicons/blob/main/src/template/mapping.json),
[Unicode EastAsianWidth.txt](https://unicode.org/Public/UNIDATA/EastAsianWidth.txt).
