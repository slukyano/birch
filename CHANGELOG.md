# Changelog

All notable changes to birch are documented here, following
[Keep a Changelog](https://keepachangelog.com/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.1] - 2026-07-28

### Added

- **Themes** (`--theme`, `birch ctl set theme`): eleven built-in looks — the `birch` flagship
  (silver-bark tree, sage icons, depth-fading guides, a single gold selection stroke), `vscode` /
  `jetbrains` / `xcode` editor mimics measured from the real apps, `mocha` / `tokyonight` /
  `gruvbox` / `nord` / `rosepine` scheme themes from their official palettes, the `retro`
  Commander (DOS-blue canvas, black-on-cyan cursor bar, `+`/`-` marks), and the ANSI-safe
  `plain`. Themes control the palette, indent-guide style, selection treatment, chevron glyphs,
  folder layout, and per-theme Nerd Font icon families.
- **Config file** at `~/.config/birch/birch.toml` (`$XDG_CONFIG_HOME` honoured; `--config <path>`
  overrides): persists the default theme, the display toggles, and `open-cmd`. Precedence is
  config < CLI flags < `birch ctl set`.
- Bidirectional CLI toggles (`--icons`/`--no-icons`, `--show-hidden`/`--hide-hidden`, …) so the
  command line overrides the config in either direction.

### Fixed

- `birch ctl reveal` now resolves paths through symlinked prefixes (e.g. macOS `/tmp` →
  `/private/tmp`) instead of rejecting them as outside the root.

### Removed

- The `files-first` sort setting (`--files-first`, `ctl set files-first`); directories always
  sort before files.

## [0.1.0] - 2026-07-24

Initial release.

### Added

- Interactive file tree: tree view, Nerd Font icons, git status badges (with ancestor rollups),
  compact folder chains, live filesystem and git updates, fuzzy filename search, picker mode
  (`--pick`), and state persistence.
- Mouse support: click selects, double-click activates, chevron toggles, hover highlight, scroll.
- Control socket and the `birch ctl` subcommand (`reveal` / `get-path` / `get-root` / `set` /
  `set-root` / `open` / `quit`) over a versioned NDJSON protocol, with walk-up instance resolution.
- Reference host adapters for tmux, herdr, and cmux ([`contrib/`](contrib)).
- Install via a Homebrew tap (`brew install slukyano/tap/birch`) and prebuilt release binaries for
  macOS (arm64 / x86_64) and Linux (x86_64).

[Unreleased]: https://github.com/slukyano/birch/compare/v0.1.1...HEAD
[0.1.1]: https://github.com/slukyano/birch/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/slukyano/birch/releases/tag/v0.1.0
