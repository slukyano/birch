---
type: Task
title: Add a flag to disable state persistence
description: A CLI flag / runtime setting to turn off remembering (and restoring) expansion, selection, and scroll per root.
status: Draft
priority: low
---

birch saves expansion/selection/scroll per root and restores it on the next launch (git-gated;
task `008-add-state-persistence`). Some users (and some transient/picker uses) want a fresh,
stateless tree each time. Add a way to turn persistence off.

Design questions: a launch flag (`--no-state` / `--no-remember` — pick a name consistent with the
other `--no-*` flags) and/or a `birch ctl set` runtime setting; does "off" mean *don't restore*,
*don't save*, or both (probably both); and whether it also has a config-file default once the
config file (`031-add-config-file`) lands. Surfaced while writing the README "Other features"
section, which currently notes this as planned.
