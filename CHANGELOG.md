# Changelog

All notable changes to birch are documented here, following
[Keep a Changelog](https://keepachangelog.com/) and [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- **Multi-selection.** Hold `Shift` with any movement key to extend a selection
  over a range of rows, or `Ctrl`-click to add a row to it or take one out.
  `Shift`-click extends by mouse wherever the terminal forwards the modifier —
  many keep `Shift` for their own text selection while an application holds the
  mouse, which birch does not try to take back. Every selected row carries the
  selection wash and the cursor keeps its accent bar, so a single selection
  looks exactly as it always did. A dimmed row can never join a selection, and
  any unmodified move collapses one back to a single row.

- **A context menu, on the right button.** It is the surface birch's actions
  live on, since printable characters are reserved for search. Arrows move
  inside it, `Enter` invokes, `Esc` closes — and `Esc` now backs out of the
  menu before it clears a search or quits. It targets the selection: a
  right-click inside one acts on all of it, a click outside collapses to that
  row, and a click below the last row targets the root. Entries whose action
  does not exist yet are absent rather than greyed out, so it currently holds
  **Open** and **Reveal Root Here** and grows as the file operations land.

- **`PgUp`, `PgDn`, `Home`, and `End`** move the selection — a page is the
  viewport less a row of overlap — and let the viewport follow. During a search
  they move by matches, like `↑`/`↓`, and stop at the ends rather than wrapping.

- **`--pick --multi`** confirms several paths and prints one per line; plain
  `--pick` still prints exactly one, so existing recipes are untouched. Add
  `--print0` to separate with `NUL`, which is the only unambiguous form when a
  file name may itself contain a newline.

- **`birch ctl get-path --all`** prints every selected path. On the wire
  `get-path` still returns exactly one path in `data` — the cursor — and the
  whole selection arrives in a new optional `paths` field, so clients that have
  never heard of it are unaffected.

- **The host adapters are on `PATH`.** `birch-cmux`, `birch-tmux`, and
  `birch-herdr` now install alongside `birch` through every channel — Homebrew,
  the shell installer, and `cargo install`, which shipped none of them before.
  So a cmux `dock.json` reads `birch-cmux dock-run` instead of an absolute path
  into a particular install, and the documented
  `bind-key b run-shell "birch-tmux toggle …"` works off a fresh install. The
  scripts remain the source of truth — each binary carries the script of the
  same name — and are read and copied from
  [`contrib/`](https://github.com/slukyano/birch/tree/main/contrib) in the repo.
  They are no longer installed separately into `share/birch/`.

- **Branch badges**: a column marking the files this branch would merge —
  everything that differs from the remote base (`origin/HEAD`, else
  `origin/main`, else `origin/master`). Collapsed folders roll it up, so a
  folded tree still shows where the branch's work is. The mark is a branch icon
  with `--icons` and `∆` without one, so it needs no font it cannot count on.
  The root row names
  the base and how long ago you last fetched, since birch does not fetch for
  you. Off with `--no-branch-badges`, the `branch-badges` config key, or
  `birch ctl set branch-badges off`.
- **Badge placement** is now yours: `--badges right|left|none|auto`, the
  `badges` config key, and `birch ctl set badges left`. Each theme picks a
  default — JetBrains and the Commander show none, matching what they actually
  do — and `auto` hands the choice back to the theme. `none` is not
  `--no-git`: names keep their status colours and folders keep their rollups,
  and the freed width returns to filenames.

- **Configurable scroll speed**: `--scroll-lines <n>` (1–10, default 3), the `scroll-lines` config
  key, and `birch ctl set scroll-lines <n>` at runtime. Out of range is refused by the flag and by
  the socket, and clamped by the config file, which never blocks launch.
- **A scrollbar** on the pane's right edge, shown whenever the rows overflow the viewport and
  hidden when everything fits. The thumb never fills the track: space above it means there is more
  above, space below means more below. Off with `--no-scrollbar`, the `scrollbar` config key, or
  `birch ctl set scrollbar off`. Its column does not accept clicks.

### Fixed

- **cmux: a preview opens in the workspace you are in.** The Dock adapter aimed
  at the window's "main pane", which it worked out as the *first* terminal pane
  in the window — so with several workspaces open, files opened in whichever one
  happened to come first, and cmux then switched you to it. `birch-cmux preview`
  now names only the window and lets cmux resolve the rest, which it does at open
  time: the selected workspace, and that workspace's focused pane.

- **Search now works from a large root.** Opening the picker in a home
  directory used to report "no matches" forever: nothing was searchable until
  a walk of every entry finished. Results now appear in well under a second
  and keep improving as the walk continues, and the status line distinguishes
  "still indexing" from "nothing matched" — so "no matches" is only said when
  it is true. On an 800,000-entry home directory, first results arrive in
  ~300 ms instead of never, and the index rests at a fraction of its former
  size ([ADR 0026](docs/adr/0026-the-index-is-a-tree-published-progressively.md)).
- **Quitting while a file is open in an editor now quits.** `birch ctl quit`
  replied "ok" and kept running, and closing the pane was ignored the same
  way; both now exit properly, saving state and restoring the terminal, and
  the editor is left to finish.

- **A burst of input no longer freezes the pane.** Every event cost a full rebuild of every visible
  row — twice per input event — behind an unbounded queue, so a fast scroll kept moving for seconds
  after the fingers stopped and answered no keystroke meanwhile. The loop now handles a whole batch
  of queued events and draws once, and scrolling no longer rebuilds the rows at all. On a
  9 000-row tree a hard flick went from 785 ms unresponsive to 4 ms, and an overscrolled burst from
  3 483 ms to 3 ms ([ADR 0024](docs/adr/0024-the-loop-draws-once-per-batch.md)).

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
