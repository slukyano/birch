---
type: Task
title: Add the config file
description: Personal defaults in ~/.config/birch/birch.toml; CLI flags override; birch-ctl set changes at runtime.
status: Done
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
TOML. `--config <path>` overrides the location with an explicit file — primarily for tests
(point at a fixture) and alternate setups. (No `--print-config-path`: the default path is fixed and
documented; a discovery flag isn't worth the surface.)

**Module.** A `config` module in **`birch-core`** (pure data — no ratatui): a `Config` struct with
`#[serde(default)]` `Option` fields mirroring the `Settings` toggles (kebab keys: `icons`, `git`,
`hidden`, `ignored`, `noise`, `compact`, `files-first`, `mouse`), plus `theme` (`ThemeId`) and
`open-cmd`. `Config::load()` reads the file; **tolerant** — unknown keys and bad individual values
are skipped and a malformed file degrades to defaults, never fatal. Any warning goes to **stderr at
startup, before the TUI takes the screen** — never surfaced inside the TUI (the launching shell /
pane host sees it; the UI stays clean).
Adds `toml` + `serde` (derive) to `birch-core`.

**Precedence** (`Settings::default()` → config → CLI flags → `ctl set`): `main.rs` stops building
`Settings` inline and instead starts from `config.to_settings()`, then applies the CLI flags. Flags
are **bidirectional** (each toggle has both `--x` and `--no-x` via clap `overrides_with`, last one
wins), so the CLI overrides config in either direction (ADR 0022). Theme resolves
`cli.theme.or(config.theme).unwrap_or(Birch)`.

**Public surface.**
- **On disk:** `$XDG_CONFIG_HOME/birch/birch.toml`, else `~/.config/birch/birch.toml`.
- **CLI:** `--config <path>` (use this file instead of the discovered one; for tests and alternate
  setups); **bidirectional toggles** — every setting flag gains its inverse (`--icons`/`--no-icons`,
  `--show-hidden`/`--hide-hidden`, `--git`/`--no-git`, …) so the CLI can override config either way.
- **Dependencies:** `toml` + `serde` (derive) added to `birch-core`.
- **TOML keys** (all optional; a missing key means "use the built-in default"):

  | key | type | maps to | default |
  |-----|------|---------|---------|
  | `theme` | string (`ThemeId`) | `Settings.theme` | `birch` |
  | `icons` | bool | `Settings.icons` | `true` |
  | `git` | bool | `Settings.git` | `true` |
  | `hidden` | bool | `Settings.show_hidden` | `true` |
  | `ignored` | bool | `Settings.show_ignored` (dimmed when true) | `true` |
  | `noise` | bool | `Settings.show_noise` | `false` |
  | `compact` | bool | `Settings.compact` | `true` |
  | `mouse` | bool | `Settings.mouse` | `true` |
  | `open-cmd` | string | open command template | unset → `$VISUAL`/`$EDITOR`/opener |

  Surface asymmetry to keep in mind: `theme` is the only *new* `ctl set` key (`025`); `mouse` and
  `open-cmd` are config- and CLI-settable but **not** `ctl set`-settable (they aren't in
  `SettingKey` today, and this sprint doesn't add them there).

**Tests:** precedence (config sets a value, a flag overrides it); tolerant parse (unknown key, bad
value, malformed file → defaults + warning); XDG path resolution; missing file → defaults.

**Docs:** a short README **Configuration** section and a note in `docs/design.md`'s defaults table.
