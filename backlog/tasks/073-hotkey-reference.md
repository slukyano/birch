---
type: Task
title: Show a hotkey reference
description: Make the key bindings discoverable in the running app - the open question is always-on footer vs. a summoned overlay, and which key can even summon it.
status: Draft
priority: medium
---

Nothing in the running app says what the keys do. `--help` lists them, and the context menu shows
accelerators beside the actions it hosts, but neither is in front of a first-time user staring at a
tree. Everything not on the menu — `Esc` layering, `→` splitting a chain, `/` switching search to
paths, the search-cycling keys — is currently folklore.

## The decision this task exists to make

**Always-on, or summoned?** They are different products and the task should pick one deliberately.

| | Always-on footer | Summoned overlay |
|---|---|---|
| Cost | One or two rows of a **narrow side pane**, permanently — the most expensive real estate birch has. | Zero until asked. |
| Discovery | Perfect: it is simply there, and can be context-sensitive (search active vs. not). | Needs a hint to exist, or it is as invisible as `--help`. |
| Precedent | mc, ranger, htop, lazygit's bottom bar. | k9s / lazygit `?`, VS Code's command palette. |
| Fit | Reads as a "TUI application"; birch is trying to read as a *panel*. | Keeps the tree the whole pane. |

A third shape worth weighing: **neither, but louder** — a one-line hint on the status line at first
launch only (or when the tree is empty), plus a `Keyboard Shortcuts` entry in
`071-add-context-menu`, on the theory that the menu is already the documented action surface.

## The sharp constraint

**Printable characters are permanently reserved for search — so `?` is not available.** The
conventional summon key for exactly this overlay is the one key birch can never bind; typing `?`
searches for a file named `?`. The candidate set is therefore the non-printables: `F1`,
`Ctrl-/` (arrives as `Ctrl-_` on many terminals and is not reliably distinguishable), `Ctrl-K`, or
no key at all with the context menu as the only door. Whichever is chosen, it must not collide with
the reserved-character rule or with the Esc back-out layering
([ADR 0012](../../docs/adr/0012-esc-backs-out.md)).

## Other things the design must settle

- **Content source of truth.** The binding table exists in `--help`, in
  [the design doc](../../docs/design.md), and in the README. A reference that drifts from the
  bindings is worse than none — the list should be generated from the same action layer the
  bindings come from, not hand-copied a fourth time.
- **Context sensitivity.** Whether the reference shows all keys always, or only those live in the
  current state (search active, menu open, inline edit in progress, `--pick` mode).
- **Turning it off.** If always-on wins, it needs a config key and a flag like every other visual
  toggle, and `birch ctl set` support to match.
- **`--pick` mode** has a different key set (Enter picks, Esc cancels) and arguably needs the hint
  more, since a picker is often someone's first contact with birch.
