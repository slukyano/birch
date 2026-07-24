---
type: Task
title: Build the core tree view (phase 0.1)
description: Tree, arrows, lazy load, icons, open-cmd with {line}, basic mouse, real-tree/render split and source interface.
status: Done
priority: high
---

Phase 0.1 of [the design doc](../../docs/design.md): navigable lazy-loading tree with Nerd Font
icons, `--open-cmd` (with `{line}` in the template contract from day one), click/scroll mouse
support — and, load-bearing, the real-tree/render split plus the sources-as-delta-streams
interface in place from the start.

Also in scope from the defaults table and keyboard section: visibility defaults (hidden
files shown, noise like `.git`/`.DS_Store` hidden, directories first) with their initial
flags (`--no-icons`, `--files-first`, `--hide-hidden`, `--show-noise`, `--no-mouse`), and
the base keys (`↑`/`↓`, `→`/`←`, Enter, `q`/`Ctrl-C`, `Esc`).

## Design

New dependencies: `ratatui` + `crossterm` (its default backend) in `birch-tui`/`birch`,
`clap` (derive) for the CLI in `birch`. Icons are a hand-rolled Nerd Font map (extension →
glyph + color) in `birch-tui` — no icon crate dependency. No notify/gix yet.

**birch-core** (no ratatui — compiler-enforced):

- `Tree`: arena of nodes (`Vec<Node>` + `HashMap<PathBuf, NodeId>` index), each node
  carrying name, real path, kind (dir/file/symlink-to-dir/symlink-to-file), load state
  (children `None` = never loaded), and sorted child ids. Sort: dirs first (`--files-first`
  flips), then case-insensitive name. The tree only mutates by applying `TreeDelta`s.
- `TreeDelta`: `Added { parent, entries }` / `Removed { path }` / `Updated { path, … }` —
  real paths throughout. `Entry` carries name + kind (a stat snapshot).
- Source contract per [ADR 0004](../../docs/adr/0004-sources-run-on-threads.md): a source
  runs on a worker thread, receives `SourceCmd` (`Expand(path)`, later more) via mpsc, and
  emits delta batches into the unified `AppEvent` channel. `FilesSource` is the first
  implementation: recv `Expand` → readdir (one level, lazy) → send `Added`. Read errors on
  a dir produce an empty expansion plus a status message, not a crash.
- `OpenCmd`: template parsing and argv construction (`{}`, `{line}`; shell-words splitting;
  default `$EDITOR {}`, else `open`/`xdg-open`). Pure and unit-tested here; execution lives
  in the binary.
- `Settings`: shared runtime-settings struct (icons, files_first, show_hidden, show_noise,
  mouse, open_cmd) — plain data, consumed by the view layer.

**birch-tui**:

- `flat_view` per [ADR 0003](../../docs/adr/0003-view-model-lives-in-birch-tui.md): pure
  view-model — flattens the expanded tree into visible rows applying visibility (hidden
  shown by default, noise `.git`/`.DS_Store`/`Thumbs.db` hidden by default), tracks
  selection **by real path** and scroll offset, and implements the keyboard semantics
  (`↑`/`↓` move; `→`/Enter expand dir; `←` collapse-or-jump-to-parent; Enter on file =
  open). Also mouse hit-testing: row → node, chevron region vs name region.
- `render`: the ratatui widget — indentation, `▸`/`▾` chevrons, icons, selection highlight,
  one-line status bar (root path; transient messages).
- `input`: crossterm event → `Action` mapping (keys per the design doc table; single-click
  file opens, single-click dir toggles, chevron-click toggles without moving selection,
  scroll = 3 rows/tick; `q`/`Ctrl-C` quit; `Esc` reserved no-op for now).

**birch** (binary):

- clap CLI: `birch [dir]` with `--no-icons`, `--files-first`, `--hide-hidden`,
  `--show-noise`, `--no-mouse`, `--open-cmd <template>`. Flags for features that don't
  exist yet (`--no-git`, …) are added by their own tasks.
- Main loop per ADR 0004: input thread + `FilesSource` thread → one `AppEvent` mpsc;
  `recv()` loop applies deltas / dispatches actions / redraws.
- Open execution: `$EDITOR`/`--open-cmd` runs with the terminal handed over (leave alt
  screen + raw mode, wait, restore, redraw) so terminal editors work in the unintegrated
  default; the `open`/`xdg-open` fallback spawns detached. Failures surface in the status
  line.
- Symlinks: shown with their target's kind; symlinked dirs expand lazily like any dir (no
  cycle guard — expansion is user-driven and bounded); broken symlinks render as files.

**Tests**: tree delta application + sorting; open-cmd template parsing; flat-view
flattening, visibility, navigation, and selection stability against a scripted fake source.
Painting and the event loop stay thin and untested.
