# Changelog

All notable changes to birch are documented here, following
[Keep a Changelog](https://keepachangelog.com/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[Unreleased]: https://github.com/slukyano/birch/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/slukyano/birch/releases/tag/v0.1.0
