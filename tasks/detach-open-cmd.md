---
type: Task
title: Detached open commands — --open-detached
description: A flag marking the custom --open-cmd as fire-and-forget (null stdio, no TUI handover), so adapter open-cmds stop blanking the tree pane.
status: Done
priority: high
---

Every custom `--open-cmd` currently runs in `OpenMode::Terminal`: birch pauses input,
leaves the alternate screen, runs the command in the foreground of its own tty, and
re-enters — correct for `nvim {}`, wrong for the adapters, whose open-cmds are
fire-and-forget scripts (`birch-cmux preview`, `birch-tmux open-in-main`). The visible
symptom: the tree pane flashes the underlying shell on every open. Maintainer naming
decision: `--open-detached` — detached from the tty (fd-level: stdio on /dev/null),
deliberately unlike shell "background", which keeps stdout on the tty. All three
adapters pass it.

## Design

- **CLI**: `--open-detached` (clap `requires = "open_cmd"`) flips the parsed
  template's mode to the existing `OpenMode::Detached` — the fire-and-forget path
  already used for GUI openers: `spawn()` with all three stdio fds on `/dev/null`,
  child reaped by a background thread, birch never suspends. No core changes beyond
  docs; `mode` is already public. Without `--open-cmd` the flag is meaningless (the
  default platform opener is already detached; the `$VISUAL`/`$EDITOR` default wants
  the tty by definition), so requiring the pair keeps the surface honest.
- **Name** (maintainer decision): *detached* as in detached from the tty at the fd
  level — deliberately not "background", which in shell terms means out of the
  foreground process group *with stdout still on the tty*. No `setsid`, no daemon
  claim; matches the internal `OpenMode::Detached`.
- **Adapters**: all three (`birch-cmux`, `birch-tmux`, `birch-herdr`) add
  `--open-detached` to their spawn command — every adapter open-cmd is a
  fire-and-forget host-CLI script, and each currently pays the TUI suspend (the
  cmux tree-pane flash; same mechanics in tmux/herdr).
- **Docs**: design-doc CLI block and adapter-promise sentence, integrations guide,
  README promise line. Errors: a spawn failure still lands in the status line; a
  detached child's own stderr is discarded by design (same as GUI openers today).
- **Known residual** (out of scope): the first preview in a cmux session still
  briefly shows the bootstrap terminal in the *preview* pane — cmux has no
  "open file into a new split" primitive, so the adapter splits a terminal and
  replaces it. Cosmetic, once per pane lifetime, and upstream-shaped.
