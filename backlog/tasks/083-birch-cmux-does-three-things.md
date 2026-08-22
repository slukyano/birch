---
type: Task
title: birch-cmux does three things
description: Reshape the cmux adapter to socket, birch, watcher — the watcher owning both root and open command — and drop the fallbacks that cannot happen.
status: Draft
priority: medium
---

The adapter accumulated a second entry point, a callback into itself, and a set of fallbacks for
conditions that do not occur. The maintainer's shape for it is three things:

1. create a socket,
2. launch birch on that socket,
3. launch a watcher that writes to that socket on workspace-change events.

Everything else is either derivable or unnecessary.

## What moves into the watcher

**The open command, not just the root.** Today birch's `--open-cmd` calls back into
`birch-cmux preview`, which re-resolves the window and runs `cmux open`. Instead the watcher sets
the whole command on birch — `cmux open {} --window <win>` — and the **`preview` verb goes away**.

**The starting state.** The watcher calls the same `ctl` commands once, on the starting workspace,
*before* reading the event stream. So `dock-run` stops resolving the root entirely: it launches
birch at `$PWD` (which cmux's dock launcher has already `cd`ed to) and the watcher corrects it
immediately. One code path for "what should birch be pointing at", exercised at startup and on
every event.

## What this deletes

| Gone | Where it is now |
|---|---|
| the `preview` verb | `contrib/birch-cmux:125-139` |
| `$0` / `self` resolution | `:42-47` |
| `shquote` | `:49-50` — its only caller was the `--open-cmd` template |
| the `set -e` defect | `:114` — a bare assignment takes the command substitution's status, so a failing lookup kills `dock-run` *before* its own fallback on `:115`. Verified against `/bin/sh`. Deleted by construction, since `dock-run` no longer resolves the root |
| `CMUX_DOCK_START_DIRECTORY` | `:115` — cmux's dock launcher already `cd`s there, so `$PWD` equals it; and when that `cd` fails (`2>/dev/null \|\| true`) `$PWD` is correct while the variable names a directory that does not exist |
| `--id-format both` | `:64` — a leftover of the `main_pane()` removal; `cmux current-window` documents no flags |
| the legacy verb ([`082`](../archive/082-cmux-legacy-verb-deprecation.md)) | `:73` — `cmux list-workspaces` → `cmux workspace list` |

## What it needs from birch

**A runtime setter for the open command**, which does not exist — `birch ctl set` covers display
settings only (`hidden`, `icons`, `theme`, `badges`, …) and `--open-cmd` is launch-only.

The maintainer chose the setter over baking the command at launch. Baking is simpler and needs no
new surface, but the window would then be fixed for that birch's life; the setter keeps a dock birch
correct if it is ever moved between windows.

This is a **public surface addition** — a socket message plus a `birch ctl` verb — so it wants its
own design and an ADR. It is to `--open-cmd` what `set-root` is to the launch root, and the socket
protocol evolves additively, so it fits the existing model rather than extending birch's promise to
adapters. Open questions: whether it also governs `--open-detached`; what birch does between launch
and the first set (today an absent `--open-cmd` falls back to `VISUAL`/`EDITOR`, which inside a Dock
pane is wrong); and whether letting a socket peer choose a command birch will execute deserves a
note, given the socket is already 0600 in a 0700 directory and therefore same-user only.

## Fallbacks: keep or drop

The maintainer's instruction is to keep only what can really happen — a misconfigured system may
error.

| Fallback | Verdict |
|---|---|
| `CMUX_BUNDLED_CLI_PATH` (`:36-40`) | **Keep, and use it harder.** Thought to be dead because the dock launcher prepends the bundle bin to `PATH` — but resolving `cmux` once and giving birch an absolute path is strictly more robust than relying on inherited `PATH`. |
| `${BIRCH:-birch}` (`:34`) | **Keep** — it is how a dev build gets used. |
| state-dir ownership check (`:55-60`) | **Keep.** Dead weight on macOS's per-user `$TMPDIR`; load-bearing on Linux with `/tmp`. Fix the `mkdir` race (`:55`, `mkdir -p -m 700`) and the message that blames ownership when `stat` merely failed (`:56-59`). |
| missing `python3` | **Error loudly.** Today `2>/dev/null` (`:73`) swallows "command not found" and the control dies with no message. |
| blanket `2>/dev/null` on cmux calls | **Trim** — keep it where output is parsed, drop it where it only hides misconfiguration. |
| `\|\| true` on `ctl set-root` (`:103`) | **Keep** — genuinely transient, and it must not kill the watcher. |

## Considered and rejected

**Reading the directory from the event payload.** `workspace.selected` carries
`payload.cwd` directly, which would remove a `cmux` call and a `python3` per event. But
`window_id` is `null` on that event and `cmux events` has no window filter, so every window's birch
would re-root to whatever workspace was selected anywhere. Correct only with a single window. Keep
the per-window lookup.

## Also worth deciding while here

`cmux open` focuses its target, so a preview steals focus from the tree. Under this design that is
`--no-focus` in one string. Currently untested either way.
