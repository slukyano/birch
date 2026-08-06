---
type: Task
title: Make scroll speed configurable
description: Lines per wheel tick becomes a setting with the full flag / config / ctl surface, bounded to a sane range instead of a hard-coded 3.
status: Draft
priority: medium
---

Maintainer request: scroll speed should be configurable — lines per scroll, with a minimum of 1 and
a sane maximum.

`input::SCROLL_LINES` is a `const isize = 3` today (the design doc's figure), consumed only by the
two wheel actions in `handle_input`. Keyboard `↑`/`↓` move one row and are not affected.

This is the preference half of [`069`](069-fix-wheel-scrolling.md), which measured the same
constant from the other side: every tick provably moves exactly 3 rows, so the freeze `069` fixes
was never a distance problem. Distance is a matter of taste, which is what makes it a setting
rather than a fix.

## Design

### The setting

`Settings` gains `scroll_lines: u8`, defaulting to **3** — the current constant and the terminal
convention, so an unconfigured birch behaves exactly as it does now. It joins the existing
precedence chain unchanged: `Settings::default()` → config → CLI flag → `ctl set` (ADR 0022).

### Range: 1–10

**Minimum 1** is the request. **Maximum 10** is chosen against how the wheel actually arrives: a
momentum burst delivers tens of events per gesture, so at the default of 3 one flick already
travels ~90 rows, and at 10 it travels ~300. Past that a single tick stops being a speed and
becomes a teleport — the knob would only be useful for overshooting. A viewport-relative cap (one
tick never exceeding a page) was weighed and rejected: it makes the same config mean different
things in two panes, and a page jump is a different feature from a scroll speed.

Out-of-range values are handled the way each surface already handles bad input:

| Surface | Out of range | Why |
|---|---|---|
| CLI flag | hard error before the TUI starts | Immediate feedback, matches how a bad `--filter` prints on stderr. Clap enforces it with `value_parser!(u8).range(1..=10)`. |
| Config file | clamped into range, with a warning string | `config.rs` is deliberately tolerant — a malformed config degrades and warns, never blocks launch. A rejected key would break that contract. |
| `ctl set` | error response, value unchanged | The socket already answers `Response::err` for an unparseable theme id. |

### Public-surface delta

Complete list — everything here is new:

- **CLI**: `--scroll-lines <n>` (1–10, default 3).
- **Config** (`birch.toml`): `scroll-lines = <n>`, optional like every other key.
- **Socket protocol**: `SettingKey::ScrollLines`, serialized `scroll-lines`, accepted by the `set`
  verb with a numeric string value.
- **`birch ctl set scroll-lines <n>`** — the client side of the same.

No new environment variable, on-disk path, or `get` verb. The addition is additive-only, as the
protocol requires: a new `SettingKey` variant, no change to any existing field. An older instance
receiving `scroll-lines` rejects it with an error response rather than misapplying it, which is the
same behaviour `Theme` had when ADR 0021 added it.

`ctl set` follows the `Theme` shape in `handle_set`: parsed before `SettingValue`, since the value
is a number rather than on/off/toggle.

### Documentation

Three places carry the settings list and all three must gain the row, or they drift:

- `docs/design.md` — the Defaults table (`| Scroll lines | 3 | --scroll-lines <n> |`).
- `README.md` — the `birch.toml` example block.
- `--help` — the flag's own doc comment.

### Relationship to `069`

Independent, and ordered after it. `069`'s coalescing applies whatever distance the setting names —
a batch of events still sums to `n` rows each — so a higher `scroll_lines` cannot reintroduce the
freeze. The two touch the same call site (`handle_input`'s wheel arms), so `069` lands first and
this task changes the constant it reads into a settings lookup.
