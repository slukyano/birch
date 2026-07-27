---
type: ADR
title: Personal defaults live in ~/.config/birch/birch.toml; precedence is config < flags < ctl set
status: Accepted
sprint: sprint-015
---

# Context

Settings are ephemeral: `Settings` is built fresh from CLI flags on every launch
(`crates/birch/src/main.rs`), and `birch ctl set` changes them only for the running instance. There
is no persistent home for personal defaults. The theme system (ADR 0021) makes this acute — a theme
you pick but can't keep isn't a feature. `docs/design.md` and task `031` have long anticipated a
`~/.config/birch/birch.toml`; this ADR pins its location, format, and precedence.

Per-root view **state** (expansion / selection / scroll) already persists, but that is a *cache*
(`$XDG_CACHE_HOME`), keyed on the real root — a different concern from global *preferences*. Config
is preferences; it does not touch the state cache.

# Decision

A **TOML** config at **`$XDG_CONFIG_HOME/birch/birch.toml`** (else `~/.config/birch/birch.toml`).

- **Keys** mirror the existing `Settings` toggles (`icons`, `git`, `hidden`, `ignored`, `noise`,
  `compact`, `files-first`, `mouse`) plus **`theme`** (a `ThemeId`) and the open command
  (`open-cmd`). All keys optional; a missing key means "no opinion, use the built-in default."
- **Precedence: `Settings::default()` → config → CLI flags → runtime `ctl set`.** Config is the new
  default source; a CLI flag present on the command line still wins; `ctl set` wins at runtime.
- **Bidirectional boolean flags** (the ripgrep / fd / bat pattern). Each toggle gains its missing
  direction — the current negatives (`--no-icons`, `--hide-hidden`, …) get positives (`--icons`,
  `--show-hidden`, …) — wired with clap `overrides_with` so **last flag wins** and the CLI can
  override config in **either** direction, with no "can't re-enable from the CLI" gap. The
  default-direction counterparts may be `hide`-flagged in `--help` to keep it uncluttered while
  still being accepted. (Back-compat is not a constraint pre-1.0.)
- **Tolerant parsing** (matches the socket's additive philosophy): unknown keys and individual
  bad values are logged to stderr and ignored, never fatal; a malformed file degrades to built-in
  defaults with a warning, it never blocks launch. Newer configs stay readable by older birch.
- **Location in the crate graph:** parsing lives in **`birch-core`** (a `config` module producing a
  partial `Settings` + `ThemeId`) — pure data, no ratatui, unit-testable. Adds `toml` + `serde` to
  core.
- A `birch --print-config-path` helper prints the resolved path (for discovery/editing).

# Consequences

- `main.rs` builds `Settings` by layering: load config → start from it (or defaults) → apply present
  CLI flags. The theme resolves the same way (config `theme` → `--theme` → `ctl set theme`).
- New dependencies in `birch-core` (`toml`, `serde` derive). Core still builds without ratatui.
- The config is **defaults-only**; it is read at startup. Live editing does not hot-reload (use
  `ctl set` for runtime changes) — a watch could come later if wanted.
- Documented in the README (a short Configuration section) and `docs/design.md`'s defaults table.

# Alternatives considered

- **JSON / YAML / RON** — TOML chosen: hand-edited, comment-friendly, ubiquitous for Rust CLIs.
- **Env vars for defaults** — rejected as a primary mechanism (opaque, undiscoverable); `$BIRCH_*`
  stays limited to the few operational knobs it already covers.
- **Store defaults in the state cache** — rejected: the cache is per-root, disposable, and keyed on
  path; global preferences are a separate, human-edited artifact.
- **One-directional flags only** (keep just the negatives) — rejected: a value config turned off
  couldn't be re-enabled from the CLI, an awkward gap for little benefit now that bidirectional is
  the common, expected pattern.
