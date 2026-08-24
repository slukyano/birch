---
type: Sprint
title: The adapter seam — a follow that survives, a failure that speaks
status: Implementing
branch: sprint/022
tasks:
- 086-adr-0016-window-scoped-framing
- 085-report-open-cmd-failures
- 083-birch-cmux-does-three-things
- 084-follow-watcher-outlives-birch
- 095-follow-watcher-never-reconnects
- 096-open-with-names-the-adapter
---

# Scope rationale

Sprint 019 declared the cmux integration finished. Live use since then has filed five defects
against it, and four of them land inside the same twelve lines — `contrib/birch-cmux:95-105`, the
follow watcher. Taken one at a time that loop gets rewritten four times; taken together it gets
rewritten once, into the shape [`083`](../tasks/083-birch-cmux-does-three-things.md) already
specifies.

The set has a single theme, and it is not "cmux". It is **what an adapter does when something goes
wrong**. Today the answer everywhere is *nothing*: a `--open-cmd` that exits non-zero is discarded
by birch ([`085`](../tasks/085-report-open-cmd-failures.md)); a follow watcher whose event stream
ends retires in silence ([`095`](../tasks/095-follow-watcher-never-reconnects.md)); a watcher whose
birch is gone runs forever, equally silently ([`084`](../tasks/084-follow-watcher-outlives-birch.md));
and every diagnostic inside the adapter is redirected to `/dev/null`. The observed symptom of all
four is the same — a tree that stops responding, with nowhere to look. The sprint's job is to make
each of them say so, and then to survive the ones that can be survived.

[`096`](../tasks/096-open-with-names-the-adapter.md), filed while this scope was being drawn, is the
same silence seen from the front: the menu's `Open` row names the adapter shim it runs rather than
the host it routes to, because argv[0] is all the label has to go on. An adapter that cannot say
what it is gets described by its filename.

The sprint stays on the one host it can run. Everything here is birch itself or `birch-cmux` — the
adapter with a live instrument behind it (`contrib/testcmux`) and a queue of observed defects. The
same class of latent fault in `birch-tmux` and `birch-herdr` is real and filed, and it waits for
[`sprint-023`](sprint-023.md), where the fix and the session that exercises it arrive together
rather than a sprint apart. The label fix splits the same way: the mechanism and `birch-cmux`'s name
land here, and `097-tmux-herdr-set-their-display-name` carries the other two lines to the sprint
that can look at them.

# In-scope task ledger

- **`086-adr-0016-window-scoped-framing`** — *low, docs plus live verification.* Restates ADR 0016's
  coupling as **per-window placement** rather than the Dock (the script makes no Dock API call at
  all), and corrects two facts that have aged out — `dock.json` controls *can* take a `cwd`, and
  config seeding loses to any restored Dock snapshot, including an empty one, which is the most
  likely first-run failure. Carries three live checks whose answers the rest of the sprint needs:
  which Dock scope a config-seeded control lands in, what a restored snapshot does to it, and
  whether `cmux current-window` returns a ref or a UUID. Open for design: whether ADR 0016 is
  amended (prior art: ADR 0007) or superseded.
- **`085-report-open-cmd-failures`** — *medium, small build, one product decision.* birch reports
  only that the *spawn* failed; the reaper does `let _ = child.wait()` and stderr goes to
  `/dev/null` (`crates/birch/src/app.rs:1448-1465`). A non-zero exit becomes a status-line message.
  Roughly ten lines. Open for design: whether stderr is captured for the message or the exit status
  alone carries it, and how the message shares the status line with its existing users.
- **`083-birch-cmux-does-three-things`** — *medium, design-heavy.* Reshapes the adapter to socket,
  birch, watcher — the watcher owning both the root and the open command, the `preview` verb going
  away, and the fallbacks for conditions that cannot occur going with it. **Needs a public-surface
  addition birch does not have**: a runtime setter for the open command, as a socket message plus a
  `birch ctl` verb. That is an ADR of its own. Open for design: whether the setter also governs
  `--open-detached`, what birch executes between launch and the first set, and whether
  `--no-focus` belongs on the open.
- **`084-follow-watcher-outlives-birch`** — *medium, small build.* Nothing signals the backgrounded
  watcher when its birch exits, so it forks a `cmux` call and a `python3` on every workspace switch
  for the life of the app. The fix is a captured parent pid and a `kill -0` guard at the top of the
  loop, which does not depend on signal behaviour. **This is a prerequisite for `095`, not a
  neighbour of it.**
- **`095-follow-watcher-never-reconnects`** — *high, small build, real diagnosis.* The watcher
  subscribes once, without `--reconnect`, so the first stream end retires it and leaves the dock
  birch frozen at its last root — the reported symptom. cmux documents the resume contract and ships
  `--reconnect` and `--cursor-file` for it. **Scoped to every mechanism that can stop follow, not to
  the likeliest one**: the reconnect, *and* a window identity that can go stale and be noticed
  rather than one captured once at `contrib/birch-cmux:109`, *and* a diagnostic channel that
  survives the control's life. The second is a defect in its own right — a watcher whose window UUID
  stops resolving loops forever doing nothing, and two docks racing `focused_window()` onto one UUID
  also contend for one socket path. The third is what keeps a fix for an unobserved trigger honest:
  a mechanism this sprint finds and cannot fix becomes its own named `Draft` task, never a silent
  remainder.
- **`096-open-with-names-the-adapter`** — *medium, small build, one decision shared with `083`.* The
  menu row reads **"Open with birch-cmux"**: the label takes the open command's argv[0] basename
  (`crates/birch-core/src/open_cmd.rs:120-124`) and all three adapters bake themselves there, so the
  row names the courier rather than the destination. The open command gains an optional display
  name, preferred over the basename when set; absent one, `nvim` still reads `nvim`. The mechanism
  and `birch-cmux`'s own name; `birch-tmux` and `birch-herdr` set theirs in
  `097-tmux-herdr-set-their-display-name`, which travels with the sprint that runs them. Open for
  design: whether the name arrives as a launch flag, through `083`'s runtime setter, or both, and
  what bounds a string that arrives over a socket and gets drawn in a menu row.

# Ordering / dependencies

- **`086`'s live checks first.** They are cheap, they are answers rather than arguments, and two of
  the three feed designs later in the sprint: whether the socket key is a ref or a UUID bears
  directly on `095`'s stale-window failure family, and what a restored Dock snapshot does decides
  whether a dock birch survives a cmux restart at all.
- **`095`'s verify-first step also runs early**, because it waits on a recurrence in live use. The
  instrumentation it needs is small and can be in place while other tasks are designed.
- **`085` next, and independently.** It is birch-side, it blocks nothing, and everything after it
  benefits: it is the channel through which the reshaped adapter reports what it could not do.
- **`083` next** — the reshape, and the ADR for the open-command setter. `084` and `095` rewrite the
  same loop, so they follow it rather than racing it.
- **`084` before or with `095`.** Ordering is not optional here: `--reconnect` removes the stream EOF
  that is today the only thing retiring a leaked watcher.
- **`096` designed with `083`, implemented after it.** They share one question — a command set at
  runtime that cannot carry its own display name would reintroduce the bad label through the new
  surface — so the answer has to be given once, for both.

# Risks

- **Six tasks, the same count sprint 021 carried**, and the count flatters the load: four of the six
  are small builds, and the weight sits in `083`'s new public surface and in verification.
- **`095` closes every mechanism it can name, and its trigger is still unobserved.** Both known
  mechanisms are in scope, so nothing is knowingly left half-fixed — but a third mechanism no one
  has thought of would survive the sprint. That is what the diagnostic channel is for, and why the
  task carries a verify-first step rather than a confident closure. If a recurrence after the fix
  names something new, it is a new task with an observation attached, not a reopening of this one.
- **`086` is a verification task wearing a documentation task's clothes.** Its three live checks can
  each turn up something worse than a stale sentence — a birch that quietly disappears after a cmux
  restart is the plausible one, and it would be a defect with no task yet.
- **`096` designs a surface for three adapters and proves it on one.** Only `birch-cmux` sets a
  display name here. If the design does not fit what `birch-tmux` and `birch-herdr` need, that is
  found in [`sprint-023`](sprint-023.md) through
  `097-tmux-herdr-set-their-display-name` — cheap to correct for a launch flag, more expensive if
  the name rides on `083`'s socket setter.

# Considered but out of scope

- **`026-add-multiple-roots`, `030`, `032`, `033`** — the source and root work. Off-theme, and each
  is a sprint's worth on its own.
- **`073-hotkey-reference`** — the discoverability gap that sprints 020 and 021 widened. Unrelated to
  the adapter seam.
- **`094-flaky-ctl-socket-test`** — the flaky test guards `crates/birch/src/ctl.rs`, which this
  sprint touches through the open-command setter. Tempting to fold in; held back because chasing an
  intermittent failure is unbounded work and would be the sprint's second diagnosis task after
  `095`. If `083`'s socket work makes the flake reproducible, the finding goes onto the task.
- **`087-tmux-herdr-bake-raw-argv0`** — the same latent argv[0] fault in `birch-tmux` and
  `birch-herdr`, fixed once in the shared carrier. Held back to keep this sprint on the one host it
  can run; planned as [`sprint-023`](sprint-023.md), where the fix and its verification arrive
  together.
- **`035-verify-herdr-integration`** — the backlog's oldest `high`, and deferred once already. It
  needs a live interactive herdr session, which is a different instrument from the throwaway cmux
  this sprint verifies against, and it would be this sprint's second host. Planned as
  [`sprint-023`](sprint-023.md) rather than deferred again, so the scope survives the gap.
- **`089`, `090`, `091`** — input and pointer follow-ups, all low, all off-theme.
- **`053`, `055`, `056`, `058`, `061`, `065`, `066`, `070`** — configuration and theme work.

# Sprint-start action

Commit `backlog/sprints/sprint-022.md` to `main`; cut branch `sprint/022` from it. The design phase
opens with `086`'s live checks and `095`'s instrumentation, both of which produce facts the later
designs consume.

# Session log

* **2026-08-23** — Sprint opened; branch cut. Design phase started with
  `086-adr-0016-window-scoped-framing`'s three live checks against `contrib/testcmux`: all three
  answered, `086` designed. `cmux current-window` returns a UUID; a config-seeded control lands in
  the window Dock (`dock:global`) and config precedence is first-match-wins, so two birch controls
  cannot be seeded into one window; a hand-started second one refuses legibly. Two findings beyond
  the checks: a dock control's `CMUX_WORKSPACE_ID` holds its own **window UUID**, which removes
  `095`'s focus race at the root, and a restored Dock snapshot brings back bare terminals rather
  than controls, so a dock birch lasts one cmux session — filed as
  `098-dock-birch-does-not-survive-a-restart`.
* **2026-08-23 (cont.)** — All six tasks designed. `085` scoped to the detached path and given a
  stderr-carrying message; `083` designed with ADR 0035 (Proposed) for the `set-open-cmd` verb, and
  simplified by `086`'s finding that the dock control knows its own window; `084` and `095` collapse
  into one supervision loop that reconnects, re-syncs on every reconnection, and checks birch's
  liveness; `096` takes a display name on both the launch flag and the setter. Maintainer decisions:
  ADR 0016 is amended rather than superseded; a preview keeps stealing focus; a failed detached open
  reports whenever the child exits.
