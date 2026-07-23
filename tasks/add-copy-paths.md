---
type: Task
title: Add copy name and paths
description: Copy Name / Relative Path / Absolute Path over OSC 52 with fallbacks - split out of the 0.5 bundle.
status: Draft
priority: medium
---

Split out of `add-file-operations` so it can ship earlier (maintainer asked about
copying during MVP feedback): the copy primitive (name / root-relative / absolute) with
the OSC 52-first clipboard chain (works over SSH and in tmux), native fallback
(`pbcopy`/`wl-copy`/`xclip`), status-line last resort. `Ctrl-Shift-C` copies the
relative path (the design doc's accelerator); the other variants arrive with the
context menu. Terminal Cmd+C stays what it is — the terminal's own selection copy
(Shift-drag under mouse capture).
