# Editor file trees — ground truth

How VS Code, IntelliJ IDEA, and Xcode actually render their file trees. Layout facts from the
editors' sources; colors pixel-sampled from live screen captures of each app (dark themes, 2026).
Basis for the `vscode` / `jetbrains` / `xcode` themes.

## Row layout — the twisty/icon column model

All three editors use the same two-column model: a disclosure-twisty gutter, then an icon column,
then the label. Files leave the twisty gutter blank so their icons align under folder icons.
The twisty never replaces the icon — with one exception: an icon theme may set
`"hidesExplorerArrows": true` in VS Code, removing the twisty column entirely.

VS Code's default Seti icon theme ships **no folder icons**, so folder rows show only the chevron
— the origin of `FolderStyle::Compact`. IDEA and Xcode always show folder icons
(`FolderStyle::Icon`).

## Measured values

### VS Code (Dark Modern)

| element | value |
|---|---|
| sidebar background | `#191a1b` |
| text (files and dirs — same color, dirs NOT bold) | `#bfbfbf` |
| indent guide | `#585858` |
| chevron | `#8c8c8c` (codicon, thin) |
| selection, inactive | `#2c2d2e` |
| selection, active (from theme JSON `list.activeSelectionBackground`) | `#04395e` |

### IntelliJ IDEA (New UI dark)

| element | value |
|---|---|
| panel background | `#191a1c` |
| file text | `#bcbec4`-family; dir text `#d1d3d9` (not bold; root bold) |
| chevron | `#b5b8be` (thin, New UI) |
| folder icon | `#c3c5ca` monochrome outline |
| selection, unfocused | `#33353a`; active (documented) `#2e436e` |
| indent guides | none drawn by default |
| VCS name-coloring | file names tinted by VCS status (e.g. excluded/untracked warm `#cd9d72`) |

### Xcode

| element | value |
|---|---|
| navigator background | `#2a2a2a` |
| row text | `#ffffff`; dirs not bold |
| chevron | `#a7a6a7` thin (post-Big Sur; filled triangles are the pre-2020 look) |
| selection, unfocused | `#464646` grey rounded pill; focused = system accent blue |
| indent guides | none |
| git status | right-aligned `A` / `M` letters — the same layout as birch's badge column |

## Key corrections these measurements forced

- Thin chevrons everywhere modern; filled `▶▼` only as a deliberate retro look.
- Directory names are not bold in any of the three editors (only the root/project row).
- IDEA New UI draws no indent guides by default; Xcode never does.
- All three use full-row selection fills, no accent bar.
