# Changelog

All notable changes to birch are documented here, following
[Keep a Changelog](https://keepachangelog.com/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.2.0] - 2026-08-05

Search and the picker became one thing. A search or a filter now **dims** rows instead of replacing
them, and a dimmed row is inert — it cannot hold the selection, a click does nothing, and `Enter`
never acts on it ([ADR 0023](docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)).

### Added

- **Glob filter** (`--filter <glob>`, repeatable) narrows the tree in both the picker and the
  everyday pane. A pattern without `/` matches the name, one with `/` the root-relative path, and
  brace expansion works (`'*.{md,txt}'`); `'*/'` means "any directory". `--filter-mode skip` (the
  default) greys out non-matching files and makes them unselectable, `hide` omits them outright —
  files only, never directories, so rows never vanish mid-browse. In `--pick`, a folder can only be
  confirmed when a pattern names it.

### Changed

- **`--pick` no longer changes what search does.** A query used to replace the tree with a flat
  match list and disable `→`/`←`; the tree now stays on screen in both modes, with the same keys and
  the same context (ancestors, siblings, git badges). The flat picker list is gone.
- **`↑`/`↓` step between matches in tree order** instead of fuzzy-score order, so the selection
  travels down and up the pane rather than teleporting. The selection anchors forward on every
  rematch — the first match at or after the current row, wrapping at the end — and stays put while
  its row still matches.
- **`→` is never a silent no-op**: it expands a collapsed directory, splits a compacted chain, or
  advances to the next live row. It does nothing only when no live row follows.
- `Esc` restores the pre-search view in both modes. With nothing live there is no selection, and
  `Enter` reports `no matches`.

### Fixed

- The selection wash no longer swallows the match highlight — lit characters kept their gold
  background on the selected row, which under the anchor rule is the current match.
- A non-matching directory no longer renders bold-and-dim, reading as prominent while inert.
- A dim folder's chevron works again: a chevron is structure, not selection, so it toggles on a dim
  row without moving the selection.
- A search no longer snaps a scrolled viewport back to the match when a filesystem change arrives
  mid-query.
- Symlinked directories are resolved when building the search index, so ordering, anchoring, and
  filtering agree with the row on screen.

### Documentation

- README and [`docs/research/nerd-font-glyphs.md`](docs/research/nerd-font-glyphs.md) record the
  fallback-font hazard behind apparent indent-guide misalignment: PUA chevron glyphs from a
  non-`Mono` Nerd Font advance wider than a cell and sit off centre, while the guide drawn by the
  primary font does not. Setting a `Mono` primary family fixes it; birch's geometry is correct.

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

[Unreleased]: https://github.com/slukyano/birch/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/slukyano/birch/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/slukyano/birch/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/slukyano/birch/releases/tag/v0.1.0
