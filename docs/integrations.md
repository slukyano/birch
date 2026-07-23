# Integrating birch into a pane host

birch is designed to run persistently in a side pane; this document is the integration
story. The whole promise birch makes to a host is four things:

1. the [control-socket protocol](adr/0011-ndjson-protocol.md) (NDJSON, versioned,
   additive-only),
2. `birch --socket <path>` — the host picks the socket path, so it never has to discover
   what it created,
3. `birch --open-cmd '<template>'` — `{}` is the path (appended when absent) — plus
   `--open-detached` when the template is a fire-and-forget script rather than a
   terminal editor: birch spawns it with null stdio instead of handing the tty over
   (adapter open-cmds want this; without it every open suspends the tree pane),
4. clean exit on SIGHUP/SIGTERM (host closes the pane → birch dies cleanly, socket
   unlinked).

Everything else — pane spawning, toggle keybindings, editor hooks — is the host's
domain. Adapters live with their host; this repo ships three reference adapters in
[`contrib/`](../contrib) as documentation of the pattern.

## The adapter pattern

An adapter is ~a screen of shell:

1. **Spawn** the side pane running
   `birch --socket <adapter-chosen-path> --open-cmd '<host open-in-main primitive> {}' --open-detached`.
2. **Toggle**: the host kills or respawns the pane; birch needs nothing (SIGHUP exit).
3. **Reverse-reveal**: the host's (or editor's) file-focus hook runs
   `birch-ctl --socket <path> reveal <file>` — the tree follows your editing, IDE-style.
4. Optionally, main-pane bindings for `birch-ctl open` / `birch-ctl get-path`.

Single-instance discipline is the host's job: one birch pane per session, keyed however
the host keys things (the reference adapters use one socket per tmux session / per user).

## Reference adapters

- [`contrib/birch-tmux`](../contrib/birch-tmux) — `open`/`toggle`/`socket` subcommands
  over `split-window`, `send-keys`, and `kill-pane`. Suggested binding:
  `bind-key b run-shell "birch-tmux toggle #{pane_current_path}"`.
  Inside tmux, birch's mouse support requires tmux mouse mode (`set -g mouse on`) — that
  is a tmux setting, not a birch bug.
- [`contrib/birch-herdr`](../contrib/birch-herdr) — the same pattern over herdr's pane
  CLI (`herdr pane split/run/close`).
- [`contrib/birch-cmux`](../contrib/birch-cmux) — **does not** follow the split-pane
  pattern above; it integrates via cmux's right-sidebar **Dock** instead. See
  [cmux (via the Dock)](#cmux-via-the-dock) below and
  [ADR 0016](adr/0016-cmux-integrates-via-the-dock.md).

## cmux (via the Dock)

cmux does not use the split-pane pattern. Its right-sidebar **Dock** (a beta feature)
hosts persistent controls declaratively, so cmux owns birch's lifecycle from a config
file — the adapter is only the glue cmux can't provide (per-window socket, preview
routing, workspace follow). Rationale and the load-bearing facts are in
[ADR 0016](adr/0016-cmux-integrates-via-the-dock.md).

**1. Enable the Dock beta.** Command Palette → search "Dock" (Beta features), or set
`rightSidebar.beta.dock.enabled` in cmux's settings. Without it the Dock is unavailable.

**2. Add a `dock.json`** — project-local `.cmux/dock.json` or global
`~/.config/cmux/dock.json`. The Dock is global, so the command names no folder; birch
roots at the active workspace and follows workspace switches from there:

```json
{
  "controls": [
    {
      "id": "birch",
      "title": "birch",
      "command": "/abs/path/to/birch-cmux dock-run",
      "env": { "BIRCH": "/abs/path/to/birch" }
    }
  ]
}
```

Open a fresh cmux window and show the Dock (the config seeds on first show, not into an
already-open Dock). birch appears with previews and a tree that tracks your workspace.

**3. `birch-cmux` verbs** — three, of which cmux and birch invoke two automatically:

| Verb | Called by | When |
|------|-----------|------|
| `dock-run [root]` | cmux, via `dock.json` | seeds the dock birch: resolve the window, start its follow watcher, `exec` birch |
| `preview <file>` | birch, as `--open-cmd` | on file open — opens the file as a tab in the window's main pane |
| `dock-socket` | you / an editor / an agent | to address this window's birch (reverse-reveal, scripting) |

**4. Reverse-reveal** — a hook computes the window's socket and calls `reveal`:

```sh
birch-ctl --socket "$(birch-cmux dock-socket)" reveal <file>
```

One dock birch per window; its socket is keyed on the window id, so `dock-socket`
resolves the same path from the dock birch and from any pane in that window. A
background watcher re-roots the tree on each workspace switch and dies with the window
or with cmux — nothing to start at login, nothing to clean up. Requires `python3` (for
the workspace-directory lookup) and the cmux CLI — on PATH, or the adapter falls back
to cmux's bundled CLI inside Dock control shells.

## Recipes without an adapter

**Forward (tree → editor)** — point `--open-cmd` at your editor's remote primitive; no
adapter needed:

```sh
birch --open-cmd 'nvim --server "$NVIM" --remote {}' --open-detached   # inside :terminal
birch --open-cmd 'emacsclient -n {}' --open-detached
birch --open-cmd 'code -r {}' --open-detached
```

These remote primitives are all fire-and-forget, hence `--open-detached`; a
blocking editor (`--open-cmd 'nvim {}'`) wants the default terminal handover
instead.

**Reverse (editor → tree)** — one autocmd calling `reveal` on buffer switch. Neovim:

```lua
vim.api.nvim_create_autocmd("BufEnter", {
  callback = function(args)
    local file = vim.api.nvim_buf_get_name(args.buf)
    if file ~= "" then
      vim.system({ "birch-ctl", "reveal", file })
    end
  end,
})
```

`birch-ctl` without `--socket` resolves the nearest instance by walking up from the
current directory, so the plain form works whenever the editor runs under the tree's
root. Set `$BIRCH_SOCKET` (or pass `--socket`) to target a host-managed instance.
An instance started with `--no-socket` is not controllable at all.

**Scripting** — the selection is a first-class output:

```sh
nvim "$(birch-ctl get-path --abs)"     # open birch's selection from any pane
nvim "$(birch --pick)"                 # transient picker (Enter picks files and dirs)
cd "$(birch --pick)"                   # works for dirs too — Enter picks the selection
```

## Text selection & mouse notes

Mouse capture disables the terminal's native drag-to-copy; hold Shift while dragging to
select text natively (or run with `--no-mouse`). Copy-path commands arrive with the
context menu in a later release; until then `birch-ctl get-path` covers the scripting
need.
