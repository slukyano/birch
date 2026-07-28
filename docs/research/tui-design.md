# TUI visual design — research synthesis

What makes well-regarded TUIs look good, distilled from design writing and the tools themselves.
Basis for the theme system's palette rules and the flagship design.

## Principles (recurring across sources)

- **Semantic slots with two ladders**, not color lists: a background ladder (base → surface
  steps, each ~5–8% lighter) and a text ladder (muted → subtext → text → bold). Every element
  maps to a slot. ([Textual design system](https://textual.textualize.io/guide/design/),
  [Catppuccin style guide](https://github.com/catppuccin/catppuccin/blob/main/docs/style-guide.md),
  [Terminal Renaissance](https://hyperbliss.tech/blog/2026.04.04_terminal-renaissance/))
- **One interactive accent; semantic colors only for status.** Everything else neutral.
  (Charm's house style; Textual's "accent = sparse emphasis".)
- **Cursor row = neutral surface fill** 1–2 lightness steps above the background — the community
  systematically replaces saturated selection bars (lazygit ports use surface0 `#313244`).
- **Mute by fg color step, not the `Dim` attribute** (renders inconsistently across terminals);
  muted text is a desaturated tint of the background hue.
- **Don't paint a background; inherit the terminal's** (lazygit, yazi, atuin, fzf). Design against
  soft dark greys (`#1a1b26`–`#2e3440`), never `#000000`; opt-in canvas painting is the exception
  (btop's `theme_background`).
- **Guides and borders at surface/overlay level** — visible when sought, invisible otherwise.
- **Hierarchy from lightness steps** (dim → default → bold), not hue count; degrade gracefully
  to 16 colors.

## Scheme quick reference (dark variants, official values)

| scheme | bg | fg | signature accents |
|---|---|---|---|
| Catppuccin Mocha | `#1e1e2e` (surfaces `#313244`/`#45475a`/`#585b70`) | `#cdd6f4` | blue `#89b4fa`, lavender `#b4befe`, peach `#fab387`, green `#a6e3a1`, red `#f38ba8`, teal `#94e2d5` |
| Tokyo Night | `#1a1b26` (highlight `#292e42`) | `#c0caf5`, comment `#565f89` | blue `#7aa2f7`, green `#9ece6a`, orange `#e0af68`, red `#f7768e`, teal `#73daca` |
| Gruvbox dark | `#282828` (bg1 `#3c3836`, bg2 `#504945`) | `#ebdbb2`, gray `#928374` | yellow `#fabd2f`, orange `#fe8019`, aqua `#8ec07c`, green `#b8bb26`, red `#fb4934` |
| Nord | `#2e3440` (nord1–3 `#3b4252` `#434c5e` `#4c566a`) | `#d8dee9` | frost `#88c0d0` `#81a1c1`, red `#bf616a`, yellow `#ebcb8b`, green `#a3be8c` |
| Rosé Pine | `#191724` (highlight `#21202e`/`#403d52`/`#524f67`) | `#e0def4`, muted `#6e6a86` | iris `#c4a7e7`, foam `#9ccfd8`, gold `#f6c177`, rose `#ebbcba`, love `#eb6f92` |

Catppuccin is the dominant port ecosystem (300+); Tokyo Night, Gruvbox, and Nord are the durable
favorites.

## File-tree specifics

- Directory color: blue/cyan accent + bold is the near-universal convention (yazi `#03a9f4`,
  Catppuccin ports `#89b4fa`) — which is exactly why a distinctive default avoids it.
- The curated look mutes icons: per-filetype rainbows clash with any palette; tools like
  tiny-devicons-auto-colors remap icons to the theme palette. Enforced in birch as
  `Theme::icon_tint`.
- Git status: green added / yellow-peach modified / red deleted / muted ignored, as fg badges,
  never a row background.

## The catalog rule (from the design review)

**Semantics are global, hues are local.** Layout, the status column, guide-dimmer-than-text, and
findable selection are invariants; every color must pass through the theme's own palette — no
global constant (like a shared devicon orange) may punch through. This is what makes eleven
themes read as one product's catalog.
