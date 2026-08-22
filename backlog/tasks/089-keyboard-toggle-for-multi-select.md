---
type: Task
title: A keyboard way to build a discontiguous selection
description: Shift with the arrows extends a contiguous range; nothing on the keyboard toggles one row in or out, so a discontiguous set can only be built with the mouse.
status: Draft
priority: low
---

`074-add-multi-selection` binds `Shift` with the movement keys to extend a **contiguous** range, and
`Ctrl`-click to toggle a single row. There is no keyboard equivalent of the toggle, so a
discontiguous selection — three files scattered through a tree — is reachable by mouse only.

This was left out of `074` deliberately rather than guessed at: the maintainer's request named
`Shift`-click, modifier-click, and `Shift` with the arrows, and no toggle key. Adding one is a
binding decision, and bindings are scarce.

## What a design has to answer

- **Which key.** Printable characters are permanently reserved for search
  ([ADR 0008](../../docs/adr/0008-q-types-into-search.md)), so the candidates are modified or
  non-printable: `Ctrl-Space` (arrives as `Char(' ')` with `CONTROL` through crossterm, though not
  in every terminal), or `Insert` — the Norton/Midnight Commander toggle-and-advance key, which
  carries file-manager rather than editor-explorer connotations.
- **Whether it advances.** Commander-style toggling moves down a row after each toggle, which makes
  building a set fast; an editor-explorer toggle leaves the cursor where it is.
- **Whether it is worth a binding at all.** The gesture exists on the mouse, and birch's mouse is on
  by default. The case for it is keyboard-only environments and the fact that every other selection
  gesture has a keyboard form.

## Related

- `074-add-multi-selection` — the selection model this would extend.
- `073-hotkey-reference` — would list whatever this binds.
