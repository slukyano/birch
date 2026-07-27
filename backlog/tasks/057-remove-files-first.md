---
type: Task
title: Remove the files-first setting
description: Drop the files-first sort toggle — a rare option no common file tree offers; directories always sort first.
status: Designed
priority: low
---

Maintainer call (Sprint 015 design): `files-first` (sort files before directories) is a toggle
essentially no file tree or manager offers — directories-first is the universal convention. Drop it
to keep the surface lean.

## Design

Remove every trace: `Settings.files_first` (birch-core) and the sort branch that reads it, the
`--files-first` CLI flag, and `SettingArg::FilesFirst` / `SettingKey::FilesFirst` (ctl client +
protocol) with their handler arm. Do **not** add a `files-first` config key (`031`). The tree always
sorts directories before files.

**Public surface (removals).**
- CLI: `--files-first` removed.
- ctl / protocol: `SettingKey::FilesFirst` removed. This is *not* additive (the protocol shipped in
  0.1.0), but it degrades gracefully — a client sending `set files-first` now gets the tolerant
  handler's unknown-setting error, not a crash — and back-compat is not a constraint pre-1.0.
- config: no `files-first` key.

Folded into Sprint 015 because it edits the exact `Settings` / `SettingKey` / CLI surface the theme
and config tasks already touch.

**Tests:** sort order is always directories-first; `--files-first` is no longer accepted;
`ctl set files-first …` errors as an unknown setting.
