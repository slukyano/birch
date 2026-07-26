# birch — modern interactive file tree for the terminal

<p align="center">
  <img src="docs/assets/logo.svg" alt="birch — modern interactive file tree for the terminal" width="680">
</p>

[![CI](https://github.com/slukyano/birch/actions/workflows/ci.yml/badge.svg)](https://github.com/slukyano/birch/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/slukyano/birch?color=8a9a5b)](https://github.com/slukyano/birch/releases/latest)
[![License: MIT](https://img.shields.io/badge/license-MIT-8a9a5b)](LICENSE)

birch is inspired by the file tree views in code editors and brings that experience to the
terminal as a standalone tool. It's purpose-built as a side view for agentic coding in tools like
herdr, cmux, and tmux, and works just as well as an everyday file explorer and picker. Written in
Rust.

![birch — a file tree, git status, and fuzzy search in the terminal](docs/assets/demo.gif)

## Features

- **Everyday file explorer and picker** — live tree, git status, auto-hides noise (`.git`, `.DS_Store`).
- **Built for agentic coding** — integrates as a side pane inside herdr, cmux, or tmux.
- **Modern UX** — mouse-native, Nerd Font icons, fuzzy search.

## Quick Start

```sh
brew install slukyano/tap/birch     # see Install for other methods

birch                               # interactive tree rooted at the current directory
birch ~/code                        # ...or at a given directory

nvim "$(birch --pick)"              # pick a file and open it
```

Arrows navigate, `→`/`←` expand and collapse, `Enter` opens a file (or toggles a directory).
Type anything for a fuzzy search. `Esc` or `Ctrl-C` quit.

## Install

**Homebrew** (recommended):

```sh
brew install slukyano/tap/birch
```

**cargo** (needs a Rust toolchain) — installs the `birch` binary only, not the `contrib/` adapters:

```sh
cargo install --git https://github.com/slukyano/birch birch
```

**From source:**

```sh
cargo build              # or: cargo run -p birch
```

A terminal with a Nerd Font gives the icons; `--no-icons` works everywhere. Git statuses need `git`
on PATH and degrade to a plain tree without it. Installed via Homebrew, the contrib adapters land in
`$(brew --prefix)/share/birch/` (not on PATH).

## Opening files

`Enter` (or a double-click) runs the open command on the selected file. By default that is
`$VISUAL {}`, else `$EDITOR {}`, else the platform opener (`open` on macOS, `xdg-open` elsewhere).
A `$VISUAL`/`$EDITOR` open runs in **terminal mode**: birch hands the terminal over and waits, so
terminal editors like `nvim` work normally; the platform opener is spawned detached.

Override the command with `--open-cmd '<template>'`, where `{}` is the file path (appended if you
omit it):

```sh
birch --open-cmd 'nvim {}'
birch --open-cmd 'code -r {}'
```

(Host adapters pass `--open-detached` to make an open fire-and-forget — see
[Host integration](#host-integration).)

## Picker

`birch --pick` turns birch into a chooser: search and navigate as usual, and `Enter` prints the
selection (a file **or** a directory) to stdout and exits. The UI stays on stderr, so stdout
carries only the picked path.

```sh
nvim "$(birch --pick)"        # pick a file to open
cd "$(birch --pick)"          # pick a directory to cd into
```

## Host integration

birch is built to live in a pane next to your editor and integrate with its host — a multiplexer
or window manager. The pattern is: the host spawns birch in a side pane; birch **opens** files
back in the host's main pane; and the host can make the tree **follow** your editor (reverse
integration, IDE-style). The whole contract birch asks of a host is small, so adapters are about a
screen of shell each.

Reference adapters ship in [`contrib/`](contrib); the full pattern and editor recipes (nvim,
emacsclient, VS Code) are in [`docs/integrations.md`](docs/integrations.md).

### herdr

[`contrib/birch-herdr`](contrib/birch-herdr) — `open` / `toggle` / `socket` over herdr's pane CLI
(`pane split/run/close`).

### cmux

[`contrib/birch-cmux`](contrib/birch-cmux) — integrates via cmux's right-sidebar **Dock**: one
birch per window, file previews open as tabs in the main pane, and the tree re-roots as you switch
workspaces ([ADR 0016](docs/adr/0016-cmux-integrates-via-the-dock.md)).

### tmux

[`contrib/birch-tmux`](contrib/birch-tmux) — a side pane over `split-window` / `send-keys`. Suggested
binding:

```tmux
bind-key b run-shell "birch-tmux toggle #{pane_current_path}"
```

Mouse support inside tmux needs tmux mouse mode (`set -g mouse on`).

### Build your own

The host contract: the [NDJSON socket protocol](docs/adr/0011-ndjson-protocol.md), `birch --socket
<path>` (the host picks the path, so it never has to discover what it created), `--open-cmd
'<template>'` with `--open-detached` for fire-and-forget opens, and clean exit on SIGHUP. To make
the tree follow the editor, a file-focus hook calls
[`birch ctl reveal`](#controlling-a-running-instance). See
[`docs/integrations.md`](docs/integrations.md).

## Controlling a running instance

Every birch instance listens on a control socket, and `birch ctl` talks to it. This is an advanced
surface — mostly driven by host adapters rather than run by hand:

```sh
birch ctl reveal src/main.rs   # select and scroll to a path — this is how the tree follows an editor
birch ctl set git off          # flip a setting: hidden | ignored | noise | icons | compact | git | files-first
birch ctl get-path --abs       # print the current selection
birch ctl quit                 # exit the instance
```

`birch ctl` finds the target instance from `$BIRCH_SOCKET`, else by walking up from the current
directory (or point it with `--socket <path>`). It speaks the additive
[NDJSON socket protocol](docs/adr/0011-ndjson-protocol.md), and it is control-only — the socket
never mutates files.

## Development

Build, test, and contribution guidance is in [`CONTRIBUTING.md`](CONTRIBUTING.md). The product and
architecture spec — including the scope fence — is [`docs/design.md`](docs/design.md).

## License

[MIT](LICENSE)
