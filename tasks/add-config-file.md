---
type: Task
title: Add the config file
description: Personal defaults in ~/.config/birch/birch.toml; CLI flags override; birch-ctl set changes at runtime.
status: Draft
priority: low
blocked_by:
- build-core-tree-view
---

The Config section of [the design doc](../docs/design.md): `~/.config/birch/birch.toml` for
personal defaults — an always-running tool with flags-only config is hostile. Precedence:
config file < CLI flags < `birch-ctl set` at runtime. Covers the settings from the defaults
table plus `open-cmd`. Config polish is sequenced "Later"; the file itself is post-MVP but
should land before any packaged release.
