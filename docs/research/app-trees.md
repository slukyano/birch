# File browsers beyond the editors — looks, icons, feel

Eight apps' file trees/browsers: key colors, icon character, and the devices behind each look.
Values from published theme/skin sources (linked). Basis for the Commander (`retro`) theme and
candidates for future themes.

| app | key values | icons | feel & devices |
|---|---|---|---|
| **macOS Finder** (dark) | folder `#5ECEFF`–`#73CFFA` (measured from Apple assets — much lighter/cyaner than commonly cited), list bg `#1e1e1e`, selection `#0059d1` w/ white text | SF Symbols, one accent tint for all sidebar glyphs | Airy, glassy. One accent hue; loud rounded selection on a quiet field. |
| **Norton/Midnight Commander** | panel `#0000AA`, files `#AAAAAA`, dirs `#FFFFFF`, cursor black-on-`#00AAAA`, marks `#FFFF55`, chrome black-on-cyan | none — ASCII purity; `╔═╗` double frames | Institutional DOS confidence: 16 flat colors, hard fg/bg swaps. Canonical source: `mc/misc/skins/default.ini`. |
| **Sublime Text** | sidebar = editor bg blended 60/40 to black (Mariana `#303841` → `#1d2227`); labels `#d6d7d9`; selection ≈ 11% white wash | minimal folder silhouettes at ~30% opacity, no chevrons | Muted, utilitarian. Sidebar as darkened echo of the editor; ghost icons. |
| **Zed** (One Dark) | panel `#2f343e`, text `#dce0e5`, muted `#a9afbc`, hover `#363c46`, selected `#454a56`, accent `#74ade8` | flat single-color folder/file glyphs | Quiet, recessed. Grey-only states; hue reserved for git + one accent. Source: `assets/themes/one/one.json`. |
| **GitHub web tree** (dark) | canvas `#0d1117`, text `#f0f6fc`, muted `#9198a1`, accent `#4493f8`; selection = neutral wash + 4px rounded accent-blue left bar; folders muted grey | Octicons: folders filled, files outline (weight contrast, not color) | Neutral, documentary. Source: `primer/primitives` dark tokens. |
| **Material Icon Theme** | default folder `#90a4ae` (Blue Grey 300); files saturated Material 300–700 (TS `#0288d1`, JS `#ffca28`) | flat filled Material glyphs (`nf-md-*` in Nerd Fonts) | Tidy, "designed": desaturated folders make colorful file icons carry all signal. |
| **yazi / superfile** (Catppuccin) | base `#1e1e2e`, dirs `#89b4fa`, hover = base-on-accent chip; superfile: subtext body `#a6adc8`, focus via border color `#b4befe`, rounded `╭╮` corners | devicon curation; flavors flatten icons to the accent | Soft, plush. Surface ladders; red background reserved for broken symlinks only. |
| **Warp** | Warp Dark: `#000000` bg, pastel ANSI (red `#ff8272`, green `#b4fa72`), one electric accent `#00c2ff` | none — hairlines and dimmed metadata instead of chrome | Modern terminal: pastels on true black, single electric accent, whisper dividers. |

## Diversity shortlist (as ranked)

1. **Commander** (implemented as `retro`) — nothing else looks like white-on-`#0000AA` with a
   black-on-cyan cursor bar.
2. **Finder** — azure folder column + loud `#0059d1` selection; an airy Apple-flavored candidate.
3. **Warp** — pastel-on-black with one electric accent; a "2024 terminal" candidate.

Sources: [mc skins](https://github.com/MidnightCommander/mc/tree/master/misc/skins),
[CGA palette](https://en.wikipedia.org/wiki/Color_Graphics_Adapter),
[Mariana scheme](https://github.com/sublimehq/Packages/blob/master/Color%20Scheme%20-%20Default/Mariana.sublime-color-scheme),
[Zed one.json](https://github.com/zed-industries/zed/blob/main/assets/themes/one/one.json),
[primer/primitives](https://github.com/primer/primitives),
[vscode-material-icon-theme](https://github.com/material-extensions/vscode-material-icon-theme),
[catppuccin/yazi](https://github.com/catppuccin/yazi),
[superfile themes](https://github.com/yorukot/superfile),
[warpdotdev/themes](https://github.com/warpdotdev/themes).
