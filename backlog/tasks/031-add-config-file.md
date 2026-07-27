---
type: Task
title: Add the config file
description: Personal defaults in ~/.config/birch/birch.toml; CLI flags override; birch-ctl set changes at runtime.
status: Draft
priority: low
blocked_by:
- 002-build-core-tree-view
---

The Config section of [the design doc](../../docs/design.md): `~/.config/birch/birch.toml` for
personal defaults — an always-running tool with flags-only config is hostile. Precedence:
config file < CLI flags < `birch-ctl set` at runtime. Covers the settings from the defaults
table plus `open-cmd`. Config polish is sequenced "Later"; the file itself is post-MVP but
should land before any packaged release.

## Design

Pulled into Sprint 015 to persist the chosen **theme** (ADR 0021); backed by its own ADR 0022.

**Location & format.** `$XDG_CONFIG_HOME/birch/birch.toml` (else `~/.config/birch/birch.toml`),
TOML. `birch --print-config-path` prints the resolved path.

**Module.** A `config` module in **`birch-core`** (pure data — no ratatui): a `Config` struct with
`#[serde(default)]` `Option` fields mirroring the `Settings` toggles (kebab keys: `icons`, `git`,
`hidden`, `ignored`, `noise`, `compact`, `files-first`, `mouse`), plus `theme` (`ThemeId`) and
`open-cmd`. `Config::load()` reads the file; **tolerant** — unknown keys and bad individual values
are warned to stderr and skipped, a malformed file degrades to defaults with a warning, never fatal.
Adds `toml` + `serde` (derive) to `birch-core`.

**Precedence** (`Settings::default()` → config → CLI flags → `ctl set`): `main.rs` stops building
`Settings` inline and instead starts from `config.to_settings()`, then applies each **present** CLI
flag (the one-directional flags override in their direction — see ADR 0022 for the documented "can't
re-enable from CLI" limitation). Theme resolves `cli.theme.or(config.theme).unwrap_or(Birch)`.

**Tests:** precedence (config sets a value, a flag overrides it); tolerant parse (unknown key, bad
value, malformed file → defaults + warning); XDG path resolution; missing file → defaults.

**Docs:** a short README **Configuration** section and a note in `docs/design.md`'s defaults table.
