---
type: Task
title: Verify the herdr integration live
description: Run birch-herdr in an interactive herdr session; check SGR mouse passthrough and reverse-reveal.
status: Draft
priority: high
tags:
- research
blocked_by:
- add-host-adapter-and-recipes
---

The reference adapter `contrib/birch-herdr` is written against herdr's CLI surface but
needs verification in a live interactive session, which automated runs cannot provide:

- SGR mouse passthrough in the birch pane (the design doc flags this as an early check
  for the flagship host) — click, scroll, and hover reporting.
- The full loop: spawn, open-in-main-pane via the adapter's `--open-cmd`, toggle
  keybinding, reverse-reveal from an editor in the main pane.
- Pane sizing/ratio ergonomics for a 30-ish-column tree.

Requires the maintainer at a terminal with a running herdr session.
