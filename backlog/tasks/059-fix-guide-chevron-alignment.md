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

**The finding that changes the outcome: the offset follows the *glyph*, not only the font.** The
`plain` theme uses base-font geometric chevrons (`▸` U+25B8 / `▾` U+25BE) instead of private-use-area
icons. Rendered in the **same non-`Mono` font** that shifts the octicon chevron by ⅓ cell, `plain`'s
chevrons stay centred and align with the guides. Every other theme uses PUA chevrons — `birch`,
`mocha`, `tokyonight`, `gruvbox`, `nord`, `rosepine` use octicons `\u{f460}`/`\u{f47c}`;
`vscode`, `jetbrains`, `xcode` use codicons `\u{eab6}`/`\u{eab4}` — and those are the glyphs whose
advance width the Nerd Font variants disagree about.

So birch **can** fix the default case from its own side, by drawing the chevron with a glyph the
base monospace font provides.

**Decision.**

1. **The `birch` theme's chevrons become base-font geometric glyphs** (`▸`/`▾`, matching `plain`).
   The default experience is then aligned in every Nerd Font variant, at no cost in surface, and the
   folder *icon* keeps its Nerd Font glyph — icons are not in a guide column, so their width does not
   produce a misalignment.
2. **The measured mimic themes keep their PUA chevrons.** `vscode`, `jetbrains`, and `xcode` exist
   to reproduce a specific editor's look, which is the codicon chevron; changing it would defeat the
   theme. Their behaviour under a non-`Mono` font is a documented caveat.
3. **The scheme themes** (`mocha`, `tokyonight`, `gruvbox`, `nord`, `rosepine`) follow `birch` — they
   are palettes, not shape mimics, so they have nothing to lose.
4. **Document the font recommendation** in the README install notes: a `Mono` Nerd Font variant, which
   additionally keeps folder and file icons inside one cell (the 1.43-cell icon is a font property no
   theme choice can fix).
5. **No new theme axis.** A per-theme guide glyph was considered and rejected: it trades one
   misalignment for another and adds a permanent field to fix a font problem.

**Open at design time.** The maintainer reports the offset while using a `Mono` font, which this
harness measures as pixel-exact — so their configuration differs from the one measured here (a
likely cause is symbol *fallback*: when the primary family lacks the PUA codepoint, the terminal
substitutes another font, which may be a non-`Mono` Nerd Font, regardless of the configured family).
Their terminal, exact font family, and size are needed to reproduce; the chevron change above is
expected to resolve it either way, because it removes the PUA chevron from the guide column
entirely.

**Public surface.** None — no flags, config keys, protocol fields, environment variables, on-disk
paths, or public APIs. Two theme glyph constants change and the README gains a font note.

**Tests.**

- Theme assertions pin the `birch` and scheme themes to the base-font chevrons and the mimic themes
  to their codicon/octicon glyphs (the existing chevron assertions in `theme.rs` are updated, so a
  future change cannot silently reintroduce a PUA chevron in the default theme).
- The capture harness (tape + measurement script) is preserved under `docs/research/` with the
  measured table, so the check is repeatable rather than a one-off.
