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
