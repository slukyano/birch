---
type: Task
title: Esc backs out — and quits at top level
description: Esc clears search or closes surfaces; with nothing to dismiss it quits. Ctrl-C always quits.
status: Done
priority: high
---

Maintainer feedback on the MVP: with `q` reserved for search (ADR 0008), Ctrl-C was the
only way out, which feels wrong for a TUI.

## Design

Per [ADR 0012](../docs/adr/0012-esc-backs-out.md): Esc is layered dismissal — it clears
an active search (restoring the pre-search view), will close the context menu when that
exists (0.5), and with nothing to dismiss it quits the app. This matches the picker,
which already quits on Esc with an empty query, and the common convention (lazygit, fzf)
of Esc backing out one level at a time. `Ctrl-C` remains the always-works quit. The
design doc's keyboard table is updated accordingly (it predates ADR 0008's `q`
resolution).

Implementation: `App::on_esc`'s no-search branch returns quit in tree mode too (it
already cancels any pending reveal). Accidental quits are cheap: expansion, selection,
and scroll persist per root and restore on relaunch.
