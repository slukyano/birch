---
type: Sprint
title: The cmux integration, finished
status: Scoped
branch: sprint/019
tasks:
- 079-preview-opens-in-current-workspace
- 051-install-adapters-on-path
- 080-verify-cmux-integration-live
---

# Scope rationale

The maintainer stated the goal directly: *"there is a birch in sidebar, it always switches to the
current workspace's cwd, it always opens previews in a new tab in the current workspace."*

Two of the three already ship. `018`, `021` and `024` built the Dock control, the workspace-selected
follow watcher, and the `preview` verb. The sprint exists because of the gap between the third and
what is built, and because "fully" means a setup that does not depend on a checkout path.

**The gap is real and was observed live.** `birch-cmux preview` targets the window's *first*
terminal pane. A cmux window holds many workspaces, so that is not the selected one: on the
maintainer's own window the selected workspace was rooted at `~/workspaces/personal/birch` while the
target resolved to `pane:3` in `workspace:3 "Group 2"`. Previews open somewhere else.

**The setup gap is why "fully" is not yet true.** `birch-cmux` is not on `PATH` — `birch-dev`
installs the adapters to `share/birch-dev/` — so both of the maintainer's `dock.json` files hardcode
an absolute path into this repository and a **debug** binary, and `docs/integrations.md` tells every
other user to do the same.

**Nothing here has been checked end to end.** Each task verified its own piece as it landed. The
three behaviours together, from an installed setup, never have.

# In-scope task ledger

- **`079-preview-opens-in-current-workspace`** — *mid, medium.* Retarget `preview` from the window's
  first terminal pane to the selected workspace's, resolving it the way the follow watcher already
  resolves the root (`cmux list-workspaces --window <id> --json`). This is the documented behaviour,
  so [`docs/integrations.md`](../../docs/integrations.md) and
  [ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md) change with it. Open for design:
  which pane when the workspace holds several (`cmux tree` marks one `[focused]`), and what happens
  when the selected workspace has no terminal pane at all.
- **`051-install-adapters-on-path`** — *mid, medium.* Make `birch-cmux` callable by name from both
  the stable formula and `birch-dev`, so a Dock config reads `birch-cmux dock-run` rather than a path
  into someone's checkout. Covers `birch-herdr` and `birch-tmux` by the same mechanism. Open for
  design: whether the adapters install as `bin` entries, and how that interacts with cargo-dist's
  generated formula — the same constraint that shaped the `birch-dev` formula.
- **`080-verify-cmux-integration-live`** — *mid, medium.* Drive a **separate** cmux instance
  (`AGENTS.md`) and confirm all three behaviours, plus `dock-socket` resolution and reverse-reveal.
  Anything unobservable is named as unobservable rather than reported as a pass.

# Ordering / dependencies

- **`079` first** — the only behavioural gap between what ships and what was asked for, and
  independent of the rest.
- **`051` next** — it changes how the integration is installed, which is what `080` should verify
  against.
- **`080` last** — it verifies the result of both.

# Risks

- **The Dock is a beta UI surface** and seeds its config on first show, not into an already open
  Dock. Whether it can be revealed from the CLI is unverified; if it cannot, `080`'s sidebar step
  needs the maintainer at the window, as `035` does for herdr.
- **`051` may be constrained by cargo-dist**, which owns formula generation. The `birch-dev` work
  already established what can and cannot be expressed there.

# Considered but out of scope

- **`035`** — high priority, and the same *shape* of task as `080`, but it needs the maintainer at a
  live interactive herdr session and is not agent-executable.
- **`071`, `028`, `029`, `034`** — the action surface, unblocked since `067` and the natural next
  sprint. Off-theme here.
- **`070`** — filter match counts; inherits the gutter model `064`/`078` left behind.
- **`026`, `053`, `073`** and the theme follow-ups (`055`, `056`, `058`, `061`, `065`, `066`) —
  off-theme.
- **`072`, `074`** — still forbidden by the scope fence in `docs/design.md`.
- **`030`, `032`, `033`** — the additional sources; "Later" in the design doc.

# Sprint-start action

Scope committed to `main`; branch `sprint/019` cut from it. Design phase opens with `079`, whose
gap is already reproduced and recorded in the task.

# Checklist

- [ ] 079-preview-opens-in-current-workspace
- [ ] 051-install-adapters-on-path
- [ ] 080-verify-cmux-integration-live

# Open questions

_(none yet)_
