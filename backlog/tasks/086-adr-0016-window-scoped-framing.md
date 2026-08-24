---
type: Task
title: ADR 0016 describes a Dock coupling the code does not have
description: birch-cmux is window-scoped, not dock-scoped; and the ADR's "verified facts" about the Dock have aged out of date.
status: Draft
priority: low
---

## The framing

[ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md) reads as though `birch-cmux` were
coupled to the Dock. It is not. The complete Dock-specific surface in the script is the verb names,
the socket filename prefix `birch-dock-`, and the `CMUX_DOCK_START_DIRECTORY` fallback at `:115` —
and [`083`](083-birch-cmux-does-three-things.md) removes that last one, leaving pure naming. Every
cmux call is generic: `current-window`, `workspace list`, `events`, `open`. There is no Dock API
call, no `--placement dock`, no `dock.json` awareness.

What actually binds the adapter to the Dock is the **follow watcher's semantics**. Re-rooting on any
`workspace.selected` is correct exactly when birch is per-window and lives *outside* any one
workspace — in a workspace split it would be re-rooting a pane you cannot see. The Dock is currently
the only cmux placement with that property, so the coupling is to **per-window placement**, not to
the Dock.

"No non-dock fallback" is accurate as a *decision* and inaccurate as a description of coupling.
Restate it: the adapter assumes one birch per **window**, living outside any single workspace; the
Dock is the only cmux placement that provides it.

## The aged facts

The ADR's "verified facts" date from sprint-011. The Dock has since grown tabs, splits, browsers,
in-app pane creation, `dock_scope`, and session persistence. Two are now wrong:

- **"The Dock is global, so `dock.json` can't name a folder"** (`contrib/birch-cmux:111-112`, and the
  same claim in [`docs/integrations.md`](../../docs/integrations.md)). `dock.json` controls take a
  `cwd` field. The design is still right — the tree should *track* the workspace, not pin a folder —
  but say "we deliberately do not pin one", not "we cannot".
- **"the config seeds on first show, not into an already-open Dock"** (`docs/integrations.md`, step
  2) is really *"seeding runs only for a Dock with no saved snapshot; a restored snapshot takes
  precedence, including an empty one"*. This is the most likely first-run failure: anyone who has
  ever opened a Dock adds the config and sees nothing. Name it, and name the escape hatch.

## Verify while here

- **Which Dock scope a `dock.json` control seeds into.** Text output marks panes `[dock:workspace]`
  or `[dock:global]`. If a birch control can land in more than one Dock in a single window, both
  compute the same `socket_for "$win"` and the second `exec birch` dies `AddrInUse`
  (`crates/birch/src/ctl.rs:96`) — leaving a bare shell where the tree should be.
- **What a restored Dock snapshot does to a config-seeded birch control** — does `dock-run` re-run,
  or does the section come back as a plain interactive shell? cmux restores "terminal session
  metadata", which is not obviously the same thing. If it is the latter, birch quietly disappears
  after any cmux restart, which nobody would have noticed while the maintainer's `dock.json` pointed
  into a checkout.
- **Whether the socket key is a ref or a UUID.** `:62` and `:66` say UUID, and so does ADR 0016, but
  `cmux current-window` documents no flags and cmux "output defaults to refs". If it is a ref, the
  comments and the ADR are wrong and `cksum` is not needed for length. Not a correctness bug —
  birch handles a stale socket (`ctl.rs:85-102`) and live refs are unique — but a leaked watcher
  ([`084`](084-follow-watcher-outlives-birch.md)) could outlive its window and then drive a
  different window's birch through a recycled ref.

## Design

All three checks were run against `contrib/testcmux` (throwaway instance, own bundle id, own
UserDefaults, own socket) on 2026-08-23. The answers below are observations, not readings.

### What the checks found

| Check | Answer |
|---|---|
| Which Dock scope a config-seeded control lands in | `[dock:global]` — the window Dock. `~/.config/cmux/dock.json` seeded both its controls there, and the global Dock renders inside whichever workspace is selected, following the selection. |
| Whether two birch controls can collide in one window | Not from config. Precedence is first-match-wins — `.cmux/dock.json` in the project or a parent, **else** `~/.config/cmux/dock.json` — never a union, so config cannot seed two. By hand it can happen, and it announces itself: the second birch prints `cannot bind the control socket: … is in use by a live instance` and the Dock drops to an interactive shell. |
| What a restored Dock snapshot does to a config-seeded control | **It replaces it with a bare shell.** See below. |
| Whether `cmux current-window` returns a ref or a UUID | A **UUID**. `--id-format both` is accepted and ignored — `cmux current-window` documents no flags. |

### The dock birch does not survive a cmux restart

Quit and relaunch, twice, with session restore enabled: the Dock snapshot restores the *panes* and
loses the *controls*. Each comes back as a plain interactive login shell titled `Terminal`, the
control command is not re-run, and `dock.json` does not re-seed, because seeding runs only for a
Dock with no saved snapshot. Nothing in the CLI puts it back — `cmux reload-config` reloads Ghostty
config and `cmux.json` and left the Dock untouched, and there is no CLI verb for reloading the Dock
config. The escape hatch cmux documents ("explicitly reloading the Dock config replaces the current
Dock with the config contents") is in-app only.

**A dock birch's lifetime is therefore one cmux session.** This is cmux's design, not a defect in
the adapter — cmux owns the Dock lifecycle, which is the whole premise of ADR 0016 — but it is the
integration's sharpest edge and nothing states it. Recorded as
[`098`](098-dock-birch-does-not-survive-a-restart.md), which owns whatever birch can do about it;
this task owns saying it.

### The correction with teeth: the control already knows its own window

ADR 0016's Context says the dock control's shell has "**no usable `CMUX_WORKSPACE_ID`** (identity
must come from `cmux current-window`)". The premise is right and the conclusion does not follow.
`CMUX_WORKSPACE_ID` is unusable *as a workspace id* — cmux's own `resolveWorkspace` rejects it, as
the sprint-019 amendment already records — because it holds the **window UUID**. Observed in a dock
control's environment: `CMUX_WORKSPACE_ID` and `CMUX_TAB_ID` both equal the exact string
`cmux current-window` returns for that window.

So a dock control can state its own window identity instead of inferring it from focus. That
matters beyond tidiness: `focused_window()` (`contrib/birch-cmux:64`) resolves the *focused* window,
which is only this control's window when this control's window is focused — an assumption that
holds when a person clicks the dock and does not hold when several controls start at once.
[`095`](095-follow-watcher-never-reconnects.md) lists that race as one of the two known ways follow
silently stops; reading the identity from the environment removes it at the root rather than
detecting it later.

Two limits on the finding, both for [`083`](083-birch-cmux-does-three-things.md) to design against
rather than for this task to settle:

- It was observed for a **global-scope** dock control. A workspace-scoped Dock may well export a
  real workspace id in the same variable, and that case was not reachable: project Dock configs sit
  behind a trust gate that needs a click, so a freshly written `.cmux/dock.json` seeds nothing until
  someone confirms it.
- `dock-socket` and any other verb called from a workspace pane still needs `cmux current-window`,
  since those callers have no dock environment. The environment is an improvement for `dock-run`,
  not a replacement everywhere.

### What changes, and where

**ADR 0016 — an `## Amendment (sprint-022)` section**, keeping `status: Accepted`. The precedent is
the ADR's own sprint-019 amendment and ADR 0007's. Nothing here reverses the decision: cmux still
integrates via the Dock, still contrib plus docs. Three corrections:

1. **The coupling is per-window placement, not the Dock.** The complete Dock-specific surface in the
   script is the verb names, the `birch-dock-` socket prefix, and the `CMUX_DOCK_START_DIRECTORY`
   fallback that `083` deletes. Every cmux call is generic. What actually binds the adapter is the
   follow watcher's semantics: re-rooting on any `workspace.selected` is correct exactly when birch
   is per-window and lives outside any one workspace, and the Dock is the only cmux placement with
   that property. "No non-dock fallback" stays true as a decision and stops describing a coupling
   that is not there.
2. **`dock.json` can name a folder; birch deliberately does not use one.** Controls take `cwd`,
   resolved from the config base — the project directory for `.cmux/dock.json`, the home directory
   for the global config. The design is unchanged and the reason changes: the tree tracks the
   selected workspace, so pinning a folder would fight the watcher.
3. **`CMUX_WORKSPACE_ID` carries the window UUID**, per the section above, replacing "no usable
   `CMUX_WORKSPACE_ID`".

**`docs/integrations.md`** — the same three, plus the restart edge in the words a reader needs:
seeding runs only for a Dock with no saved snapshot, so a Dock that has ever been open ignores a
newly added config, and a restored Dock returns bare terminals rather than controls. Both escape
hatches are in-app.

**`contrib/birch-cmux:20-30`** — the "Design facts about the cmux Dock (all verified)" block repeats
"the Dock is global, so `dock.json` can't name a folder" (`:111-112`) and the identity claim at
`:22-24`. Both go with the code that `083` rewrites.

### Public surface delta

**None.** No CLI flag, config key, socket field, environment variable, on-disk path, or public API
changes. This task edits an ADR, a document, and comments.

### Settled

ADR 0016 is **amended**, not superseded — maintainer decision, sprint-022 design phase. An
`## Amendment (sprint-022)` section carrying the three corrections above; `status` stays `Accepted`.
The decision the ADR records is untouched, only its description was wrong, which is what the
sprint-019 amendment already did to the same ADR.

