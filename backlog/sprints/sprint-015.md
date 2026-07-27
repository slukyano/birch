---
type: Sprint
title: Visual design — earn "beautiful"
status: Designing
branch: sprint/015
tasks:
- 054-refine-tree-visual-design
- 025-add-visual-styles
- 052-fix-reveal-symlink-canonicalization
---

# Scope rationale

The publication arc is closed and birch looks the part on the outside (logo, tagline, README). This
sprint makes the **tree itself** beautiful — the render layer, seen every second of use. It pairs
the baseline visual design (`054`) with the selectable presets that build on it (`025`), and folds
in one high-priority DX bug (`052`) surfaced by the Sprint 014 review.

All visual work is **render-layer** (paint-time): no new features, verbs, modes, or hotkeys
(printable characters stay reserved for search), and the real-tree/render split holds.

# In-scope task ledger

- **`054-refine-tree-visual-design`** — *medium, design-heavy.* The baseline aesthetic: tree/indent
  guides (highest-ROI — make it read as a *tree*, not an indented list), a curated muted palette,
  softer selection/focus (background + left accent bar), breathing room (margins, padding, aligned
  git gutter), and type treatment (bold dirs, tasteful dim/italic). Aesthetic calls need maintainer
  iteration.
- **`025-add-visual-styles`** — *medium, design-heavy.* Selectable presets — `default` (Nerd Font),
  `vscode` (no folder icons, tighter), `plain` (no icons, the no-Nerd-Font fallback) — bundling the
  render toggles established by `054`. Absorbs `--no-icons`; exposes a `birch ctl set` key. Carries
  a scope-fence question (a settings surface, not pure polish) and a config-file (`031`) boundary.
- **`052-fix-reveal-symlink-canonicalization`** — *high, trivial (bug).* `birch ctl reveal
  /tmp/...` is wrongly rejected as "outside the root" on macOS because `/tmp` is a symlink;
  canonicalize both sides before the root-containment check. Independent of the visual work.

# Ordering / dependencies

- `054` lands the baseline aesthetic **first**; `025` layers presets on top of it (presets select
  bundles of the toggles `054` defines, so it cannot sensibly precede `054`).
- `052` is independent and can be designed/implemented at any point.

# Considered but out of scope

- **`031-add-config-file`** — presets (`025`) are a settings surface, but this sprint exposes a
  `ctl set` key only; a persisted config format is a separate, larger task.
- **`029-add-file-operations` / `034-add-open-with`** — context-menu/UI work, but they are *features*,
  not render-layer polish; different theme.
- **`053-add-state-persistence-toggle`** — a small settings flag, but unrelated to visual design.
- **`026`/`027`/`028`/`030`/`032`/`033`/`035`/`051`** — feature or integration work, off-theme.

# Sprint-start action

Commit `backlog/sprints/sprint-015.md` (status `Designing`) to `main`; cut branch `sprint/015`. The
design phase then designs `054`, `025`, `052` one by one on the branch.

# Checklist

- [ ] 054-refine-tree-visual-design
- [ ] 025-add-visual-styles
- [ ] 052-fix-reveal-symlink-canonicalization

# Open questions

_(none yet — design phase populates this)_

# Session log

- Scoped and cut: three tasks — the baseline tree visual design (`054`) and the presets that build
  on it (`025`), plus the high-priority reveal-symlink bug (`052`) from the Sprint 014 review.
  Branch `sprint/015` cut from `main`.
