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
