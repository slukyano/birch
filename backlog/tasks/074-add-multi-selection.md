---
type: Task
title: Support multi-selection
description: Finder-style selection of several rows with Shift and Ctrl, by mouse and keyboard - requested by the maintainer; currently forbidden by the scope fence, so it needs a fence amendment before design.
status: Draft
priority: medium
---

Maintainer request: select more than one row the way Finder does — Shift-click for a range,
modifier-click to toggle individual rows, Shift with the arrow keys to extend from the keyboard.

## Scope-fence conflict (decide first)

[The design doc](../../docs/design.md) forbids this twice, and the fence is binding:

- **"Permanently out of scope: … multi-select, bulk operations"** (the identity section);
- **"No multi-select"** (file operations).

The fence cannot be amended for the gesture alone: a multi-selection that no action consumes is
decoration, and every action that *does* consume it is a bulk operation — the next item on the same
list. So the amendment has to say how far it goes. A defensible middle stop is **read-only
plurality**: multi-select feeds Copy Paths and `--pick` output, while the mutating ops (delete,
rename) stay strictly single-target. That keeps "bulk operations" out while making the selection
useful. Deciding where the new line sits is a maintainer call and warrants an ADR.

## What a design has to answer

- **Selection becomes a set, everywhere.** Selection is currently a single real path threaded
  through the view model, state persistence (`~/.cache/birch/<root-hash>.json`), the socket, and
  the picker. All four change shape, and persistence keys on real paths, so a restored multi-selection
  must survive rows that no longer exist.
- **The picker contract is a public surface.** `--pick` prints one path on stdout; multi-pick means
  either several lines (and every consumer, including the shipped adapters, must be ready for them)
  or an explicit opt-in flag. Same question for `birch ctl get-path`, which is protocol and evolves
  additively only.
- **Dimmed rows are inert.** [ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)
  says a dimmed row cannot hold the selection and cannot be acted on. A Shift-click range that
  spans dim rows must therefore *skip* them rather than include them — and the anchor itself can be
  dimmed out from under the selection by the next keystroke.
- **The context menu scopes to the set**, which means entries are enabled or disabled by how many
  rows are selected — the first place the read-only-plurality line becomes visible to a user.
- **Compacted chains.** A chain renders several real directories as one row; whether selecting it
  selects the tail or every segment has no obvious answer.

## Terminal constraints (these bound the design, not the other way round)

- **Cmd is not deliverable.** macOS terminals keep Cmd for themselves, so Finder's ⌘-click has no
  equivalent; the toggle modifier has to be Ctrl (or Alt), and Ctrl-click is a *right*-click on
  macOS by long convention — which collides with `071-add-context-menu`.
- **Shift-click arrives** in the SGR mouse encoding's modifier bits, and **Shift-arrow** arrives as
  a modified sequence (`CSI 1;2A`) in most modern terminals — but not all, and not under every tmux
  configuration. A binding that silently does nothing in someone's terminal is a support burden;
  the design should name the floor it assumes.
- **Shift-drag is reserved** for terminal text selection (the documented mitigation for mouse
  capture), so a Shift-drag *range* selection is not available.

## Related

- `028-add-copy-paths` — the obvious first consumer: several paths, newline-joined.
- `072-drag-to-move` — dragging a whole selection is the natural follow-on ask; the two share a
  fence amendment.
- `071-add-context-menu` — where a multi-selection's available actions become visible.
