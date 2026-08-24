---
type: Sprint
title: The other two adapters — fixed, and run
status: Planned
branch: sprint/023
tasks:
- 087-tmux-herdr-bake-raw-argv0
- 097-tmux-herdr-set-their-display-name
- 035-verify-herdr-integration
---

# Scope rationale

`birch-tmux` and `birch-herdr` have a filed defect and no verification. This sprint gives them both
at once, which is the reason they travel together: fixing an adapter in one sprint and running it in
the next means the fix ships blind, and running an adapter without fixing what is known to be broken
in it wastes the session.

Neither has ever been exercised in a live host. `contrib/birch-herdr` was written against herdr's
CLI surface in sprint 004, shipped as a reference adapter, and verified only by reading.
`birch-cmux` got exactly that treatment and then generated two sprints of live-use defects — sprint
019, and [`sprint-022`](sprint-022.md) after it. The argument for running herdr before it
accumulates the same queue is that queue.

[`sprint-022`](sprint-022.md) adds a third task by leaving one behind. It builds the display name
that stops the menu naming the adapter instead of the host, and sets it only for `birch-cmux` —
the host it can run. `097-tmux-herdr-set-their-display-name` carries the same one-line change to
these two, where a live session sees the row rather than an assertion about it.

# In-scope task ledger

- **`087-tmux-herdr-bake-raw-argv0`** — *low, small build.* `contrib/birch-tmux:65` and
  `contrib/birch-herdr:57` write `$0` into birch's `--open-cmd` with no resolution, so a bare or
  relative invocation produces a callback that need not resolve from birch's cwd — `birch-cmux`
  resolves it, the other two do not. Fixed once in the shared carrier
  (`crates/birch/src/adapter.rs:28-32`), which keeps ADR 0027's argv[0] decision and leaves the
  script text runnable and comparable to what ships. Open, and a maintainer call rather than a
  cleanup: `birch ctl socket-path <key>`, which would move the private state directory, the
  ownership check, the `stat` platform split, and the socket-length hashing out of three scripts and
  into birch — against `docs/design.md`'s "four things and nothing else".
- **`097-tmux-herdr-set-their-display-name`** — *low, trivial.* `contrib/birch-tmux` and
  `contrib/birch-herdr` still produce `Open with birch-tmux` and `Open with birch-herdr`, because
  `096-open-with-names-the-adapter` set the new display name for `birch-cmux` only. One line per
  script, through whichever surface that design settled on — which makes this the first real use of
  that surface by an adapter other than the one it was designed against. Blocked by
  `096-open-with-names-the-adapter`, which `sprint-022` delivers.
- **`035-verify-herdr-integration`** — *high, verification.* Three things, in a live interactive
  herdr session: SGR mouse passthrough in the birch pane (click, scroll, hover reporting, which
  `docs/design.md` flags as the early check for the flagship host); the full loop — spawn,
  open-in-main via the adapter's `--open-cmd`, the toggle keybinding, reverse-reveal from an editor
  in the main pane; and pane sizing for a 30-column tree. Blocked by
  `010-add-host-adapter-and-recipes`, which is `Done`.

# Ordering / dependencies

`087-tmux-herdr-bake-raw-argv0` and `097-tmux-herdr-set-their-display-name` first, in either order —
they touch different lines of the same two scripts and neither depends on the other. Both land
before `035-verify-herdr-integration`, so the verification runs against the adapter as it will ship
rather than against one known to be wrong, and covers three changes at once: the argv[0] fix, the
label fix, and the adapter's own long-unverified surface.

# Expected shape

The first work is not the verification. It is half an hour on whether a throwaway herdr instance can
be driven the way `contrib/testcmux` drives cmux. Sprint 019 scoped the cmux verification as
"needs the maintainer at a terminal" and `contrib/testcmux` was the result, so the assumption is
worth testing before it books anyone's time. If it holds, the sprint grows a
`contrib/tests/herdr-integration.sh` beside the cmux one; if it does not, the sprint is a sitting
with the maintainer and should be scheduled as one.

Findings become tasks. A verification sprint that turns up no defects delivers three flipped statuses
and a session log; one that turns up defects delivers a queue, and whether those are fixed here or
scoped into the sprint after is decided then, not now.

`birch-tmux` gets no equivalent live session in this scope. It takes both fixes and tmux is the host
with the least surface to get wrong, but naming that is more honest than implying two hosts are
covered. A right-click in a tmux-spawned birch is enough to read its `Open` row, and that much is
worth doing while `097-tmux-herdr-set-their-display-name` is open.

# Scope at start

Per the workflow, a `Planned` sprint's scope is re-checked against the backlog and re-presented for
approval when it starts, so anything filed between now and then is weighed in rather than assumed
out.
