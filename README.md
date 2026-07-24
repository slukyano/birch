# birch

Lean and beautiful interactive file tree for the terminal.

birch does exactly what an IDE file tree does, and nothing else: it is the
IDEA / VS Code explorer panel, living in a terminal pane next to your editor.
Tree view, Nerd Font icons, git status badges, compact folder chains, live
filesystem updates, fuzzy find, mouse — designed to run persistently in a side
pane (herdr, cmux, tmux) or as a transient picker.

**Status:** the MVP is shipped as **v0.1.0** — tree, git decorations, live updates, fuzzy
search, picker mode, state persistence, and the control socket. File operations, content
search, and a config file are next (see the [backlog](backlog/index.md)). Full product and
architecture spec: [`docs/design.md`](docs/design.md).

## Install

**Homebrew** (recommended):

```sh
brew install slukyano/tap/birch
```

**cargo** (needs a Rust toolchain) — installs the `birch` binary (with `birch ctl`) only, not the
`contrib/` adapters:

```sh
cargo install --git https://github.com/slukyano/birch birch
```

**From source:**

```sh
cargo build              # or: cargo run -p birch
```

`mise run dev` builds and drops into a subshell with the debug `birch` and the `contrib/` adapters
on PATH — for trying the CLI as installed without shadowing a system birch.

A terminal with a Nerd Font gives the icons; `--no-icons` works everywhere. Git badges need `git`
on PATH and degrade to a plain tree without it. Via Homebrew the contrib adapters land in
`$(brew --prefix)/share/birch/` (not on PATH).

## Usage

```
birch [<options>] [<dir>]     # interactive tree rooted at <dir> (default: cwd)
birch --pick [<dir>]          # picker: Enter prints the selection (file or dir)
birch ctl <verb> [...]        # control a running instance over its socket
```

Keys: arrows navigate, `→`/`←` expand/collapse, Enter opens (via `$VISUAL`/`$EDITOR` or
`--open-cmd`), any printable character starts a fuzzy search. `Esc` backs out — search first, then
the app; `Ctrl-C` always quits. The mouse works: click selects, double-click opens/toggles (a
dir's chevron toggles on a single click), scroll to scroll — hold Shift while dragging to use the
terminal's native text selection.

```sh
nvim "$(birch --pick)"        # transient picker
birch ctl reveal src/main.rs  # make the tree follow your editor
nvim "$(birch ctl get-path --abs)"
```

Flags mirror the defaults table in the design doc (`--no-git`, `--hide-ignored`, `--no-compact`,
`--hide-hidden`, `--files-first`, …); `birch ctl set` changes them at runtime. `--open-cmd
'nvim {}'` templates how files open.

## Pane-host integration

birch's promise to a host is small: the NDJSON socket protocol, `--socket`, `--open-cmd` (with
`--open-detached` for fire-and-forget open scripts), and clean exit on SIGHUP. Reference adapters
for tmux, cmux, and herdr live in [`contrib/`](contrib), and
[`docs/integrations.md`](docs/integrations.md) has the pattern plus editor recipes (nvim,
emacsclient, VS Code).

## Development

Build, test, and contribution guidance is in [`CONTRIBUTING.md`](CONTRIBUTING.md). birch is
developed in maintainer-approved sprints, tracked as [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/tree/main/okf)
bundles — the backlog in [`backlog/`](backlog/index.md) and the docs (including decision records)
in [`docs/`](docs/index.md); the process is [`backlog/workflow.md`](backlog/workflow.md).

## License

[MIT](LICENSE)
