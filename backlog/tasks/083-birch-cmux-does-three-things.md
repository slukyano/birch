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

## Design

### The shape

Three things, in this order, and nothing else:

```sh
sock="$(socket_for "$win")"          # 1. a socket path for this window
follow_watcher "$win" "$sock" &      # 3. started first, corrects state below
exec "$birch" --socket "$sock" \
    --open-cmd "cmux open {} --window $win" --open-name cmux --open-detached "$PWD"
```

`$win` comes from the environment, not from focus — see below. `$PWD` is the dock launcher's own
directory, so `dock-run` resolves no root at all; the watcher corrects it before anyone can look.

### The window identity comes from the environment

[`086`](086-adr-0016-window-scoped-framing.md) verified that a dock control's `CMUX_WORKSPACE_ID`
holds its own **window UUID** — byte-identical to what `cmux current-window` returns for that
window. So `dock-run` reads its identity instead of inferring it:

```sh
win="${CMUX_WORKSPACE_ID:-$(focused_window)}"
```

This is not a tidy-up. `focused_window()` returns the *focused* window, which is this control's
window only when this control's window is focused — true when a person clicks the dock, false when
several controls start at once. That race is one of the two known ways follow silently stops
([`095`](095-follow-watcher-never-reconnects.md)), and reading the environment removes it rather
than detecting it.

`focused_window()` stays, for two reasons: it is the fallback above, and `dock-socket` genuinely
needs it — that verb is called from a workspace pane, which has no dock environment. The
`--id-format both` at `:64` goes, verified as accepted-and-ignored.

### The open command is baked *and* settable

The launch command above is already correct, because the window is known at launch. The runtime
setter is not there to fill a gap; it is there so a dock birch that is ever moved between windows
stays correct — the reason the setter was chosen over baking in the first place. Both, not either.

That answers the task's open question about what birch executes between launch and the first set:
nothing wrong. There is no interval where the command is a `VISUAL`/`EDITOR` fallback, and no need
for an "unset" state or a flag to express one.

**The mode travels with the command.** A setter that changed the template but not
`Terminal`/`Detached` would let an adapter hand the dock pane's terminal to `cmux open`, which
corrupts the screen. The wire carries an optional flag; absent leaves the mode unchanged.

**Nothing about the open command is persisted.** Settings persist (`birch-core/src/persist.rs`); a
host-specific open command must not, or a birch launched later under a different host inherits a
command for a host that is not there.

### Public-surface delta

A new socket verb, its `birch ctl` form, and one launch flag. All additive; ADR 0011's rules hold —
unknown fields ignored, existing fields never redefined.

| Surface | Shape | Notes |
|---|---|---|
| `Verb::SetOpenCmd` | `{"v":1,"verb":"set-open-cmd","value":"<template>","name":"<text>","detached":<bool>}` | `value` reuses the existing field. `name` and `detached` are new optional fields |
| `birch ctl set-open-cmd` | `birch ctl set-open-cmd <template> [--name <text>] [--detached\|--no-detached]` | Absent `--name` leaves the display name unchanged; absent detached flag leaves the mode unchanged |
| `--open-name <text>` | Launch flag beside `--open-cmd` | [`096`](096-open-with-names-the-adapter.md)'s display name, and the launch half of the same answer |

**Why a new verb rather than a new `SettingKey`.** `Verb::Set` would be a smaller diff — it already
carries `setting` and `value`, and `SettingKey` already has members whose values are words rather
than booleans. It is refused on two grounds. `set` is the display-settings verb and is today
incapable of causing birch to execute anything; folding an executable command into it changes what
the whole verb means, and a reader of the protocol would have no signal that one key is unlike the
rest. And `set` feeds persistence, which the open command must stay out of.

**On a socket peer choosing a command birch will execute.** The socket is `0600` inside a `0700`
directory, so the peer is the same user, who can already run anything birch could. No privilege
boundary is crossed and no mechanism is needed — but the ADR says so out loud, because "a client can
make birch exec this" is the kind of sentence that should be found in a document rather than
inferred from a mode bit.

### What this deletes

As the task's table specifies: the `preview` verb, `$0`/`self` resolution, `shquote`, the `set -e`
defect at `:114` (deleted by construction, since `dock-run` resolves no root), the
`CMUX_DOCK_START_DIRECTORY` fallback, `--id-format both`, and the legacy `cmux list-workspaces`
spelling. The fallbacks keep the verdicts already recorded: `CMUX_BUNDLED_CLI_PATH` kept and used
harder (resolve `cmux` once, pass an absolute path), `${BIRCH:-birch}` kept, the state-dir ownership
check kept with the `mkdir` race and the misleading `stat` message fixed, missing `python3` made
loud, blanket `2>/dev/null` trimmed to where output is parsed, and `|| true` kept on `ctl set-root`.

### Tests

- Protocol round-trip for `set-open-cmd` with each field present and absent.
- A `set-open-cmd` request changes what a subsequent open runs, including the mode; a request
  without `detached` leaves the mode alone; a request without `name` leaves the display name alone.
- An unknown extra field on the request is ignored (ADR 0011's additive rule, guarded by a test).
- `contrib/tests/cmux-integration.sh` gains a check that the dock birch's open command names the
  window it belongs to, which is what the whole reshape is for.

### Decisions

1. ~~Does the preview steal focus?~~ **Settled: it does.** Maintainer decision, sprint-022 design
   phase. `cmux open` focuses its target and that behaviour stays — pressing Enter on a file takes
   you to it, the way an IDE explorer does. No `--no-focus` in the open command.
2. **Whether `--open-name` is worth a launch flag at all**, given `birch ctl set-open-cmd --name`
   exists. It is, for [`097`](097-tmux-herdr-set-their-display-name.md): `birch-tmux` and
   `birch-herdr` bake their command at launch and have no watcher to set anything afterwards, so
   without the flag they cannot fix their label at all.
