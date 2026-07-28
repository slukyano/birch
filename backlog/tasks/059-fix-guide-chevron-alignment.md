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
