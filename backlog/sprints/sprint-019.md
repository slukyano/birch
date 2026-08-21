---
type: Sprint
title: The cmux integration, finished
status: Done
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

- [x] 079-preview-opens-in-current-workspace
- [x] 051-install-adapters-on-path
- [x] 080-verify-cmux-integration-live

# Open questions

_(none yet)_

# Sprint summary

The integration now does the three things it promised, and a script proves it.

**`079` turned out to be a deletion.** The adapter computed a "main pane" — the first terminal pane
in the window — because it predated `cmux open`'s ability to resolve a target itself. Reading cmux's
source settled the design: given only `--window`, cmux picks the window's *selected* workspace and
that workspace's *focused* pane, on its main actor, at open time. Passing an explicit `--workspace`
would have been strictly worse, since any id the adapter reads is stale by the time `open` runs. The
explicit `--pane` [ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md) prescribed turned
out to be a workaround for something `--window` already handles: it is the flag that stops
`cmux open` inheriting the caller's surface, which is what dragged files into the dock.

**The premise of `080` was wrong, and that was the sprint's real find.** It was scoped as
"probably needs the maintainer", the way `035` does for herdr. Both halves of that gave way:

- **A second cmux can be run.** The instance check keys on the *bundle identifier*, so a copy that
  carries its own identifier launches alongside the real one — and inherits none of its
  UserDefaults, which means its own settings and its own session. `AGENTS.md`'s `open -na cmux`
  recipe never worked and has been corrected.
- **The Dock is scriptable.** `new-surface --placement dock` creates the control, `send` runs the
  adapter in it, and `capture-pane` reads birch's rendered tree back as text. Nothing needs a
  `dock.json`, which also side-steps the project-config trust gate.

So the sprint ends with nine assertions rather than a checklist, and they discriminate: run against
the adapter as it stood before `079`, exactly the two preview checks fail.

**`051` had no clean answer, only a least-bad one.** cargo-dist installs declared binaries to `bin`
and sweeps everything else into `pkgshare`, and offers no formula template or `post_install` to
change that. The adapters reach `bin` only if cargo-dist believes they are binaries — so now they
are, each a three-line bin target carrying its script via `include_str!`
([ADR 0027](../../docs/adr/0027-adapters-ship-as-binaries.md)). The scripts still ship as scripts
beside them, so nothing about the contrib-as-example story changes.

# Task ledger

| Task | Weight | What changed |
|---|---|---|
| [`079-preview-opens-in-current-workspace`](../archive/079-preview-opens-in-current-workspace.md) | mid | A preview from the dock birch opens in the workspace you are in, in its focused pane. Previously it opened in the window's first terminal pane — a different workspace — and cmux then switched you to it. Delivered as designed. |
| [`051-install-adapters-on-path`](../archive/051-install-adapters-on-path.md) | mid | `birch-cmux`, `birch-tmux`, `birch-herdr` are on `PATH` from every channel, so `dock.json` reads `birch-cmux dock-run` and the documented tmux binding works. `cargo install` ships them for the first time. Shape **A** of four, chosen by the maintainer. |
| [`080-verify-cmux-integration-live`](../archive/080-verify-cmux-integration-live.md) | mid | A committed harness that drives a throwaway cmux and asserts all three promises plus `dock-socket` and reverse-reveal. Scoped expecting a manual checklist; delivered fully automatic. |

# Public-surface delta

| Surface | Addition |
|---|---|
| Installed binaries | `birch-cmux`, `birch-tmux`, `birch-herdr` alongside `birch` |
| Docs | `dock.json` uses bare `birch-cmux dock-run`; `BIRCH` is now only an override |

No flags, config keys, or socket messages added. No breaking changes: the adapters' verbs and
birch's adapter contract are untouched, and the scripts still ship at `share/birch/`.

# Architectural decisions

- **[ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md)** — amended. The `preview`
  bullet's "window's main pane (routed with an explicit `--pane`)" was wrong in both halves; the
  decision to integrate via the Dock is unchanged.
- **[ADR 0027](../../docs/adr/0027-adapters-ship-as-binaries.md)** — the adapters ship as binaries
  that carry their script, because that is the only way cargo-dist puts them on `PATH`.

# Bugs found & fixed

- **The one this sprint existed for**: previews opened in the wrong workspace, and moved you there.
- **Two harness bugs**, found by the harness disagreeing with itself: there is no CLI verb that
  closes a preview surface, so cleanup silently did nothing and a stale surface was read as the new
  one; and `down`+`enter` only means "open that file" when the workspace has a single row —
  with a folder sorted first, `enter` expanded it.
- **`AGENTS.md`'s cmux testing recipe** did not work and would have silently sent any future agent
  at the maintainer's own instance.

# Review findings

The independent review of the diff found three majors, all fixed:

- **The release archive would have collided.** `include` places files in the archive root under
  their *basename* — exactly where cargo-dist puts binaries — so `contrib/birch-cmux` and the new
  `birch-cmux` executable both wanted `<archive>/birch-cmux`. Every outcome was bad: the binary
  wins and `share/birch/` never exists, the script wins and `bin/birch-cmux` is a shell script with
  the shim never running, or cargo-dist errors and the release breaks. The reviewer proved the flat
  layout from the actual published v0.2.0 tarball. `include` is dropped — ADR 0027 removed the
  reason it existed — and the **five** documents asserting the scripts still ship to `share/birch/`
  were corrected. This sprint's own verification had admitted the release path was "not run end to
  end" without naming the risk.
- **`README.md` contradicted `CHANGELOG.md` inside the same diff** — still saying `cargo install`
  ships "the `birch` binary only" and that the adapters are "not on PATH".
- **The harness's riskiest `cmux` calls were its only unguarded ones.** The adapter runs inside the
  dock shell and uses bare `cmux`, including the *mutating* `cmux open`, relying on env propagation
  nothing asserted — the same failure that had already created stray workspaces once this sprint.
  And a failed check did not stop the run, so a misdirected adapter would have proceeded into the
  rounds that open files. Both fixed.

Ten minors, all fixed, of which two were more than tidying: `current_exe()` resolves symlinks and
pinned birch's `--open-cmd` to a Cellar path that `brew cleanup` deletes under a running dock birch
(now argv[0], which is what the scripts always expected); and `testcmux`'s socket guard was
tautological while `CONTRIBUTING.md` presented it as the safety mechanism.

The reviewer could not independently confirm any cmux CLI surface — it was barred from driving a
live instance — so everything about `capture-pane`, `--placement dock`, and the JSON keys rests on
this sprint's own runs.

# Remaining limitations & highlights

- **The harness is not in `cargo test`** and not in CI: it needs a GUI session and a 314 MB copy of
  the cmux bundle. It is run deliberately — `contrib/tests/cmux-integration.sh`.
- **The adapter depends on a resolution order cmux documents only in code.** If cmux changed what an
  `open` with only `--window` resolves to, previews would move silently. The harness is the guard,
  and the reasoning is recorded at the call site.
- **`contrib/` is now a build input of `crates/birch`**: editing an adapter needs a rebuild before
  the installed binary changes. The scripts stay directly runnable, so the two can be compared.
- **The Dock beta remains a beta.** Everything here rides observed behaviour of an unreleased
  surface.
- **The release path is still unexercised.** The formula's `bin.install` list is derived from cargo
  metadata, which now declares four binaries, but no release has been cut since. The next one is
  where `share/birch/` disappearing and the adapters appearing in `bin` actually shows up.

# Verification

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — green;
  **230 tests**.
- `contrib/tests/cmux-integration.sh` — **9 passed, 0 failed**, against the built `birch-cmux`
  binary, so ADR 0027's `$0` handling is exercised by the `--open-cmd` round trip.
- Counter-checked at `82635d0` (pre-`079`): 7 passed, **2 failed** — the two preview assertions,
  previews landing in `workspace:1` whatever was selected.
- **The maintainer ran the integration by hand**, in their own cmux, and confirmed all three
  behaviours. That is the end-to-end check the automated suite stands in for.
- `shellcheck -s sh` clean on `contrib/birch-cmux`, `contrib/testcmux`, and the test script.
- All three adapter shims produce byte-identical output and exit codes to their scripts.
- The generated Homebrew formula will carry them: `cargo metadata` now declares four bin targets
  for the `birch` package, and cargo-dist's `homebrew.rb.j2` emits `bin.install` over every
  executable in the archive fragment. Not run end to end — that needs a real release.

# Session log

- Scoped and cut: `079`, `051`, `080`. Branch `sprint/019` cut from `main`.
- Design: `079` read out of cmux's Swift sources rather than guessed; `051` presented as four shapes
  with cargo-dist's limits established, maintainer chose **A**; `080` initially designed around a
  manual checklist, **rejected by the maintainer**, then redesigned once a throwaway instance and a
  scriptable Dock were found. ADR 0016 amended, ADR 0027 proposed.
- Implementation: `079` (the `--pane` lookup deleted), `051` (three bin targets carrying their
  scripts), `080` (`contrib/testcmux` + `contrib/tests/cmux-integration.sh`, nine assertions).
- **Incident**: with `CMUX_SOCKET_PATH` empty the cmux CLI falls back to the default socket, and two
  stray workspaces were created in the maintainer's own instance. Both removed immediately;
  `testcmux` now refuses any socket outside its own directory.
