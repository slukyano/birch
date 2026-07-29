---
type: Task
title: Indent guides look misaligned under wide Nerd Font glyphs
description: The guide stem is cell-centered, but oversized PUA chevron/icon glyphs render right-of-center, so guides appear ~1/3 cell to the left of the chevron.
status: Done
priority: medium
---

Reported from live use: the indent guides sit slightly to the **left** of the chevron's visual
centre.

## Measurements

From a maintainer screenshot (Ghostty, ~16 px cells):

| element | ink span | centre | vs. cell centre (43) |
|---|---|---|---|
| guide `│` (level 1) | x 42–43 | 42.5 | centred, correct |
| chevron (same column) | x 40–56 (17 px) | 48.0 | **+5 px**, overflows the cell |
| folder icon (next column) | x 67–89 (**23 px**) | 78.0 | ~1.4 cells wide |

The same rows rendered with **JetBrainsMono Nerd Font Mono** (17 px cells): chevron ink 11 px,
centre 42; guide stem 42 — pixel-exact. Deeper level: chevron centre 76, guide 76.

## Diagnosis

birch's geometry is correct: guides, chevrons, and icons all occupy the same cell column
(`depth * INDENT_WIDTH`), and the guide glyph is centred in its cell. The offset comes from the
**font**: in non-`Mono` Nerd Font variants the private-use-area glyphs (octicon chevrons, folder
icons) keep their natural width — wider than one cell — so the terminal draws them shifted right
of the cell centre while the box-drawing `│` stays centred. Net visual offset ≈ 1/3 cell.
Purely cosmetic; hit-testing and layout are unaffected.

## Options to weigh

- **Document the font recommendation** (cheapest): a `Mono` Nerd Font variant fixes the chevron
  *and* the oversized folder icons at once. Belongs in the README install notes.
- **Move guides to the icon column** — the icon is the visually dominant glyph, but it is oversized
  too, so this only trades one misalignment for another.
- **Offer a guide glyph choice** per theme (e.g. `▏` U+258F sits at the cell's left edge, closer
  to where an oversized glyph starts) — a theme axis, not a fix.
- **Accept**: terminals address whole cells only; sub-cell alignment cannot be corrected from
  birch's side.

Related: `058` (terminal-palette adaptation) — both are "birch looks different in the user's
terminal than in the reference render", but this one is glyph metrics, not colour.

## Design

**A reproducible harness replaced the single screenshot.** birch is rendered by `vhs` at a fixed
font family, size, and cell grid, captured to PNG, and measured pixel-wise: for every glyph, the ink
span, its centre, and the offset from its cell's centre. Font family is a parameter, so variants are
compared like for like instead of by eye.

**Measurements** (JetBrainsMono Nerd Font, 32 px, 21.0 px cell, `birch` theme):

| variant | guide `│` offset | chevron offset | guide↔chevron | folder-icon ink width |
|---|---|---|---|---|
| **…Nerd Font Mono** | −0.5 px (−0.02 cell) | −0.5 px (−0.02 cell) | **0.0 px** | 21 px = 1.00 cell |
| **…Nerd Font** (non-`Mono`) | −0.5 px (−0.02 cell) | **+6.5 px (+0.31 cell)** | **7.0 px ≈ ⅓ cell** | 30 px = 1.43 cells |

This reproduces the reported ⅓-cell offset exactly, and confirms the geometry is not birch's: the
guide sits dead-centre in its cell in both variants, and only the chevron moves.

**The reported render measures as the non-`Mono` case.** The maintainer's own screenshot (cmux, no
font configured, so the terminal's defaults apply) was measured with the same method — the cell grid
recovered from the glyph pitch rather than assumed:

| quantity | measured | reference |
|---|---|---|
| cell width | 16.75 px | — |
| indent guide stem | centre 19.5 px | — |
| parent chevron ink | centre 24.5 px | **+5.0 px = +0.30 cell** from the guide |
| folder icon ink | 23 px wide | **1.37 cells** |

**The font files themselves confirm the cause** (measured with `fontTools` on the actual files on the
reporting machine, not inferred). cmux embeds two fonts: **JetBrains Mono Regular**, unpatched, as
the text font, and **Symbols Nerd Font** (not the `Mono` variant) for symbols. The primary font
contains **no** Nerd Font codepoint at all, so every birch icon and chevron is drawn by the fallback.

Per glyph, with the cell defined by the text font's own advance (0.6 em):

| glyph | font that draws it | advance | ink width | ink centre vs cell centre |
|---|---|---|---|---|
| guide `│` U+2502 | JetBrains Mono (primary) | 1.00 cell | 0.17 cell | **+0.00 cell** |
| `▸` U+25B8 | JetBrains Mono (primary) | 1.00 cell | 0.52 cell | **+0.01 cell** |
| octicon chevron U+F460 | Symbols Nerd Font | 1.67 cells | 0.60 cell | **+0.42 cell** |
| codicon chevron U+EAB6 | Symbols Nerd Font | 1.67 cells | 0.55 cell | **+0.37 cell** |
| folder U+E5FF | Symbols Nerd Font | 1.46 cells | 1.46 cells | +0.23 cell |
| rust icon U+E7A8 | Symbols Nerd Font | 1.33 cells | 1.33 cells | +0.17 cell |

Every Nerd Font codepoint birch uses is 1.08–1.67 cells wide in that fallback. The guide is centred
because a different font draws it. The rendered offset (+0.30 cell) is smaller than the font's
+0.42 because the terminal constrains oversized icon glyphs toward the cell; it reduces the offset
without removing it.

**A correction to the earlier reading.** The `Mono` / non-`Mono` distinction is *not* about advance
width: both installed JetBrainsMono Nerd Font variants advance exactly 1.00 cell for every glyph
here. The difference is the **ink outline** — non-`Mono` draws the octicon chevron 0.60 cell wide
centred at **+0.42**, while `Mono` draws it 0.36 cell wide at **+0.06**, and shrinks the folder icon
from 1.46 cells to exactly 1.00. And neither installed variant is in use on the reporting machine,
because cmux uses its own embedded pair.

**A glyph change was considered and rejected.** The `plain` theme's base-font chevrons (`▸` U+25B8 /
`▾` U+25BE) stay centred in the very font that shifts the octicon by ⅓ cell, so moving every theme
to them would hide the symptom. Maintainer decision: **the chevron shape is a design choice and must
not be picked to work around a font defect.** Sprint 015 chose these shapes deliberately, and they
stay: octicons `\u{f460}`/`\u{f47c}` for `birch` and the scheme themes, codicons
`\u{eab6}`/`\u{eab4}` for the measured mimics.

**Decision.**

1. **No render change, no theme change, no new theme axis.** birch's geometry is correct at every
   depth in every variant measured; the guide is centred in its cell and only the substituted glyph
   moves. A per-theme guide glyph would trade one misalignment for another.
2. **Document the font requirement** where it is actionable: the README install notes state that the
   icon set needs a Nerd Font **that the terminal's primary family provides**, in a **`Mono`**
   variant. A terminal whose primary font carries no icon codepoints falls back to a symbols-only
   font whose glyphs are 1.1–1.7 cells wide, and no birch-side change can correct that.
3. **Give the concrete fix for the reported setup**: set the primary family to a `Mono` Nerd Font so
   the fallback never runs. On the reporting machine `JetBrainsMono Nerd Font Mono` is already
   installed and measures +0.06 cell for the chevron and 1.00 cell for icons; the setting is
   `fontFamily` in `~/.config/cmux/cmux.json`, which is currently unset. The plain non-`Mono`
   variant is *not* a fix — it keeps the +0.42 cell chevron ink.
4. **Keep the measurement harness** under `docs/research/` — the tape, the measuring script, and the
   measured table — so any future report is answered with numbers instead of impressions.

**Verified.** Setting the terminal's primary family to the installed `Mono` build resolved it in
the reported environment. The setting is **not** a cmux one — cmux exposes no font key at all
(`cmux-settings list-supported`), and delegates fonts to Ghostty, so the fix is
`font-family = JetBrainsMono NFM` in `~/.config/ghostty/config`. The family name matters: Nerd
Fonts abbreviates its families, so the installed `Mono` build is `JetBrainsMono NFM` and not
"JetBrainsMono Nerd Font Mono"; an unresolvable name fails silently back to the substitute.

**Known upstream.** The class of defect is tracked in Ghostty, whose bundled fallback is
"Symbols Nerd Font" (not the `Mono` build) beside an unpatched JetBrains Mono primary:

- [discussion 8822](https://github.com/ghostty-org/ghostty/discussions/8822) — Nerd Font glyphs
  taking two cells and losing alignment with regular characters, answered with the icon
  scaling/alignment PR series ([8563](https://github.com/ghostty-org/ghostty/pull/8563),
  [8847](https://github.com/ghostty-org/ghostty/pull/8847),
  [8580](https://github.com/ghostty-org/ghostty/pull/8580) for macOS);
- [issue 9076](https://github.com/ghostty-org/ghostty/issues/9076) — scale groups derived from a
  wrong assumption about codepoint offsets, naming **the Octicon chevrons** (birch's default
  chevron family) as scaled and aligned incorrectly. Closed 2025-10-11;
- [discussions 7204](https://github.com/ghostty-org/ghostty/discussions/7204) and
  [13298](https://github.com/ghostty-org/ghostty/discussions/13298) — fallback metrics differing
  from the primary font's, same symptom from other codepoints.

So no new report is warranted from birch: the behaviour is known, tracked, and partly fixed
upstream, and the user-side remedy is a one-line font setting. Ghostty requires a discussion before
an issue ([3558](https://github.com/ghostty-org/ghostty/issues/3558)) should that change.

**Public surface.** None — no flags, config keys, protocol fields, environment variables, on-disk
paths, public APIs, or theme values. Documentation only, plus research artifacts.

**Tests.**

- No behaviour changes, so no new unit tests. The existing chevron assertions in `theme.rs` already
  pin every theme's glyphs and keep this decision from being undone by accident.
- The verification is the live cmux check described above, with the measured before/after numbers
  recorded in the task and in `docs/research/`.
