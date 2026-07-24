---
type: Task
title: Add a demo recording to the README
description: An asciinema or GIF near the top of the README showing the tree, search, and git badges.
status: Designed
priority: medium
---

birch is a visual tool and the README undersells it in prose. Add a short recording
(asciinema cast or GIF) near the top showing the tree, Nerd Font icons, git status badges,
compact chains, and fuzzy search in motion. Design: format (asciinema vs. GIF), where the
asset lives, and keeping it reproducible.

## Design

A short demo **GIF** near the top of the README — tree, Nerd Font icons, git badges, compact
chains, fuzzy search. Produce it reproducibly with **vhs** (charmbracelet) from a committed
`.tape` script; the asset lives at `docs/assets/demo.gif`. GIF over asciinema so it renders inline
on GitHub with no external dependency.

⚠️ Generating the GIF needs `vhs` and a real `birch` in a pty — the maintainer likely runs the
`.tape` locally (headless capture is impractical for a TUI); the `.tape` script and the produced
`demo.gif` are committed.
