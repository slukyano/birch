---
type: Task
title: Indent guides look misaligned under wide Nerd Font glyphs
description: The guide stem is cell-centered, but oversized PUA chevron/icon glyphs render right-of-center, so guides appear ~1/3 cell to the left of the chevron.
status: Draft
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

Both numbers are the non-`Mono` signature (measured above: +0.31 cell, 1.43 cells) and neither is
close to the `Mono` one (0.00 cell, 1.00 cell). So the reported configuration draws the
private-use-area glyphs with non-`Mono` metrics whatever the configured family is — the expected
cause is **symbol fallback**: when the primary family has no glyph for a PUA codepoint, the terminal
substitutes another font, and that fallback is not a `Mono` variant.

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
2. **Document the font requirement** where it is actionable: the README install notes state that
   the icon set needs a **`Mono`** Nerd Font variant, which also keeps folder and file icons inside
   one cell — the 1.37-cell icon is a font property no birch-side change can correct.
3. **Give the concrete fix for the reported setup**: name the terminal-side setting that pins the
   family to a `Mono` variant, so the fallback never runs. In cmux that is `fontFamily` in
   `~/.config/cmux/cmux.json`, which is currently unset.
4. **Keep the measurement harness** under `docs/research/` — the tape, the measuring script, and the
   measured table — so any future report is answered with numbers instead of impressions.

**To verify during implementation.** The harness renders through `vhs`, which is not the reported
terminal, so it cannot confirm point 3 by itself: with a non-Nerd primary family `vhs` substitutes a
correctly-centred glyph, i.e. it does not reproduce the fallback that the screenshot shows. The
check therefore runs in a **separate cmux instance** (per `AGENTS.md`: `open -na cmux`, targeted by
`CMUX_SOCKET_PATH`, never the maintainer's own), before and after setting `fontFamily`, with the
screenshot measured both times.

**Public surface.** None — no flags, config keys, protocol fields, environment variables, on-disk
paths, public APIs, or theme values. Documentation only, plus research artifacts.

**Tests.**

- No behaviour changes, so no new unit tests. The existing chevron assertions in `theme.rs` already
  pin every theme's glyphs and keep this decision from being undone by accident.
- The verification is the live cmux check described above, with the measured before/after numbers
  recorded in the task and in `docs/research/`.
