---
type: Sprint
title: Pointer feel
status: Done
branch: sprint/017
tasks:
- 069-fix-wheel-scrolling
- 075-configurable-scroll-speed
- 067-select-on-mouse-up
- 068-add-scrollbar
---

# Scope rationale

Sprint 015 made the tree look right, sprint 016 made the keyboard behave; this one is the mouse.
All three items came out of live use, and all three land in the same two places — the mouse
mapping in `input.rs` and the viewport geometry that `hit_test` and `render` share.

The ordering argument is what makes the set worth doing now rather than later: **the click model is
about to become load-bearing for two much larger features.** `071-add-context-menu` has to decide
whether right-click moves the selection, and `072-drag-to-move` has to separate a drag from a click
by a threshold — both on top of whatever press-vs-release rule holds. Settling that rule here, in a
sprint whose whole surface is the pointer, is cheaper than settling it inside the menu's design.

`069` is the only unblocked high-priority item in the backlog and the only defect: the wheel runs
away and heavy overscrolling reads as a freeze. It leads.

No new sources, no new modes, no actions. New public surface: `075`'s scroll-speed setting and
`068`'s scrollbar toggle, each a flag, a config key, and a `ctl set` key.

# In-scope task ledger

- **`069-fix-wheel-scrolling`** — *bug, high, design-light but investigation-heavy.* Retitled
  during design to **"Input bursts freeze the pane"**: measurement moved the diagnosis off the
  wheel entirely. One event cost a full O(all visible rows) rebuild — twice per input event —
  behind an unbounded queue, so any burst froze the pane (1 000 `Down` keypresses froze it
  identically to 1 000 wheel events). Peek-loading, the stated suspect, was ~18 %. Fixed by
  batching the loop (**ADR 0024**) and serving scrolling from a cached row count: 3 483 ms → 3 ms.
- **`075-configurable-scroll-speed`** — *minor, medium.* Added to scope during the design phase, at
  the maintainer's request. Lines per wheel tick stops being `input::SCROLL_LINES = 3` and becomes
  `Settings::scroll_lines`, bounded 1–10, with the full flag / config / `ctl set` surface. The
  preference half of `069`, which proved the distance was never the defect.
- **`067-select-on-mouse-up`** — *mid, design-heavy, medium.* Selection moves from button-down to
  button-up so a click reads as deliberate rather than twitchy. `map_event` maps
  `MouseEventKind::Down(Left)` to `InputAction::Click` and discards `Up` entirely
  (`crates/birch-tui/src/input.rs:89`). Open for design: whether the chevron toggle keeps press
  while the name keeps release, what a press that leaves its row before releasing does (birch does
  not track the press row today), and how `ClickTimer`'s 450 ms window is re-based so a
  double-click is still two complete clicks. Designed as **ADR 0025**: the press arms, the release
  completes, and only on the same row and the same zone — so a click can be revoked by sliding off.
  One rule for every affordance; the chevron does not keep button-down. ADR 0015 stands except for
  its moment.
- **`068-add-scrollbar`** — *mid, medium.* A scroll indicator down the pane's edge, hidden when
  everything fits, with a way to turn it off. The right two columns are the badge gutter today
  (`render::BADGE_WIDTH`), so the layout must give it a column or share one — and whatever it takes,
  `hit_test` must account for. Pure indicator in this version; dragging the bar needs mouse-drag
  tracking birch has never had, and belongs with `072`. Designed as one column at the **far right**,
  pushing the badges left and keeping the gutter, reserved only while shown; the column is inert to
  clicks, which reserves the drag gesture for later.

# Ordering / dependencies

- **`069` first** — it is the defect, it is high priority, and its likely fix (bounding what one
  frame consumes, or rate-limiting peeks) changes how the viewport moves before `068` draws a bar
  that reports viewport position.
- **`075` after `069`** — both touch the wheel arms of `handle_input`; `069` fixes the loop, then
  `075` turns the constant it reads into a settings lookup.
- **`067` second** — independent of the other two, but its ADR outcome is the thing `071` and `072`
  are waiting on, so it should not be the item that gets dropped if the sprint runs long.
- **`068` last** — the only one that touches layout, and it inherits whatever `069` settles about
  scroll behaviour. It competes with `064` (badge placement) and `070` (match counts) for the
  right-hand columns; landing first means those two inherit this layout.

# Considered but out of scope

- **`071`, `028`, `029`, `034`** — the context menu, copy paths, and file operations: the action
  surface. The natural next sprint, and it wants `067`'s click model settled first.
- **`072`, `074`** — drag-to-move and multi-selection. Both are forbidden by the scope fence in
  `docs/design.md` and cannot be designed until the fence is amended, which is a maintainer
  decision plus an ADR. `067` is a prerequisite for both regardless.
- **`073`** — hotkey reference; its leading candidate surface is a context-menu entry, so it wants
  `071`.
- **`061`, `064`, `065`, `066`, `058`, `055`, `056`** — the sprint-015 theme follow-ups; still a
  coherent set for their own sprint. `064` deliberately deferred past `068` so the scrollbar fixes
  the right-edge layout first.
- **`070`** — filter match counts; blocked on `027` shipping in live use, and a third competitor for
  the right-hand columns.
- **`030`, `032`, `033`** — the additional sources; "Later" in the design doc.
- **`026`** — multiple roots; needs a dedicated design phase.
- **`035`** — high priority but not agent-executable; requires a live interactive herdr session.
- **`051`** — packaging; needs verification against a real `brew install`.
- **`053`** — off-theme; a rider for a future config/settings sprint.

# Sprint-start action

Scope committed to `main`; branch `sprint/017` cut from it. Design phase opens with `069`, where
reproduction precedes design.

# Checklist

- [x] 069-fix-wheel-scrolling
- [x] 075-configurable-scroll-speed
- [x] 067-select-on-mouse-up
- [x] 068-add-scrollbar

# Open questions

_(none open — `069`'s two were settled: the batch is bounded by a ~8 ms time budget rather than an
event count, and the loop contract is recorded as ADR 0024.)_

# Sprint summary

The mouse became trustworthy, and the loop underneath it stopped being the bottleneck.

**`069` was not the task it was written as.** The report was wheel-specific — "runs away sometimes,
overscrolling makes it all freeze" — and the file named viewport-driven peek-loading as the leading
suspect. A PTY harness feeding synthetic SGR wheel events, measuring how long a keystroke waits
behind a burst, moved the diagnosis entirely: peeks accounted for ~18 % (`--no-compact`), git for
none (`--no-git`), and a burst of 1 000 `Down` keypresses froze the pane identically at 3 516 ms.
The defect was the event loop — one event cost a full O(all visible rows) rebuild, paid twice per
input event, behind an unbounded channel — and the wheel was merely the only device that emits
hundreds of events per second. The task was retitled **"Input bursts freeze the pane"** so the
archive records what was actually wrong, and the loop contract became
[ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md).

**`067` settled a question two unbuilt features were waiting on.** Moving the click to the release
is a feel fix on its own, but `071` (does right-click move the selection?) and `072` (where does a
drag threshold live?) both build on whatever press-vs-release rule holds, and settling it inside the
context menu's design would have been more expensive.
[ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md) gives them a contract to cite.

**`075` arrived mid-sprint** at the maintainer's request and is the only scope growth.

# Task ledger

| Task | Weight | What changed |
|---|---|---|
| [`069-fix-wheel-scrolling`](../archive/069-fix-wheel-scrolling.md) | **major** | An input burst froze the pane; the loop now handles a batch of queued events and draws once, and scrolling reads a cached row count instead of rebuilding rows. Planned as peek rate-limiting; measurement disproved that suspect and the task was retitled from "Mouse-wheel scrolling feels broken". |
| [`075-configurable-scroll-speed`](../archive/075-configurable-scroll-speed.md) | mid | Rows per wheel tick became a setting (1–10, default 3) across flag, config, and socket. Created during the design phase at the maintainer's request; not in the approved scope. |
| [`067-select-on-mouse-up`](../archive/067-select-on-mouse-up.md) | mid | A click became a press and a release on the same row and zone, acting on the release. Delivered as designed; the one open question — whether the chevron keeps button-down — resolved to a single rule for every affordance. |
| [`068-add-scrollbar`](../archive/068-add-scrollbar.md) | mid | A one-column indicator at the right edge, shown only when rows overflow, inert to clicks. Delivered as designed. |
| [`076-search-unusable-on-large-roots`](../tasks/076-search-unusable-on-large-roots.md) | — | Created this sprint from a maintainer report, `Draft`, not worked: from `$HOME` the index never lands and the status line claims "no matches". |
| [`077-quit-swallowed-during-terminal-handover`](../tasks/077-quit-swallowed-during-terminal-handover.md) | — | Created this sprint from the independent review, `Draft`, not worked: a pre-existing bug outside the diff. |

# Public-surface delta

| Surface | Addition |
|---|---|
| CLI | `--scroll-lines <n>` (1–10), `--no-scrollbar` (and hidden `--scrollbar`) |
| Config (`birch.toml`) | `scroll-lines`, `scrollbar` |
| Socket | `SettingKey::ScrollLines`, `SettingKey::Scrollbar` — additive only |
| `birch ctl` | `set scroll-lines <n>`, `set scrollbar <on\|off\|toggle>` |

`069` and `067` add no public surface. No breaking changes. The binding spec
([`docs/design.md`](../../docs/design.md)) was amended to match: both settings joined the Defaults
table, and the Mouse section gained the release rule and lost the claim that a chevron acts on the
press.

# Architectural decisions

- **[ADR 0024](../../docs/adr/0024-the-loop-draws-once-per-batch.md)** — an iteration of the loop
  handles a *batch* of events and draws once, bounded by an ~8 ms budget. Every event is still
  handled, in order; only the frame is deferred. Quitting and terminal hand-off close a batch at
  once, and an event computes the row set only if it needs one.
- **[ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md)** — a click is a press and a
  release on the same row and zone, taking effect on the release; anything else abandons it. The
  double-click window measures release to release. ADR 0015 stands except for its moment.

# Bugs found & fixed

Found by the **independent review** — no majors. Nine minor findings and two nits: all nine minors
and one nit (the selection wash) are fixed below; the other nit was that the backlog had not been
closed out yet, which this section's own commit did.

- A hand-off in the event that *opened* a batch let exactly one queued event run behind the child —
  the flag was checked after handling, not before. Both call sites now share one tested rule.
- `scroll-lines` above 255 failed to parse as a `u8` and discarded the **whole config file**,
  clamping only below that. Read as the widest integer TOML carries, so 300 clamps to 10.
- *(Withdrawn.)* A status message arriving mid-batch was reported as losable — cleared by a later
  event in the same batch and never drawn. A guard was written, then removed: with an unreadable
  directory expanded from inside a sustained wheel burst, the message reached a frame on every run
  both with and without the guard. The finding was reasoned rather than observed, and no
  reproduction exists, so the special case was not kept.
- `thumb()` returned a full-height thumb for a one-row track, claiming top and bottom at once.
- The selection wash painted over the scrollbar column, erasing the track on the selected row in
  the themes where the guide colour equals the wash.
- `FlatView::scroll_by` had no caller left while the app duplicated its clamp.
- The ADR 0015 doc comment had been separated from `resolve_click` and still described the chevron
  as acting on the press.
- Six code paths had no test: the socket `scrollbar` key, the config `scrollbar` branch, arming
  from a hit, the batch rule, a pane too narrow for the furniture, and the wash boundary.
- `README.md`'s `ctl set` key list and `CHANGELOG.md`'s `Unreleased` section were missing this
  sprint's four user-visible changes.

Found during implementation, by a property test over many row/viewport combinations: `thumb()`
panicked when the track had a single free slot (a 2-row viewport), where three states cannot be
shown. The bottom stays exact there and the top slot is shared.

# Remaining limitations & highlights

- **Keyboard navigation is still O(rows) per event.** `Down` must skip dim rows, so it cannot use
  the scroll fast path: 3 ms at a realistic 50 keys/s on a 9 156-row tree, but 1 382 ms under a
  synthetic 2 500/s burst — a rate no keyboard produces. ADR 0024 records the shape of this
  ("per-event work is now worth auditing; the linear-in-rows cost stands"); the two figures are
  measured here and appear nowhere else.
- **Intermediate frames are not drawn.** A 300-event flick renders the destination, not the
  journey. This is the intent — those frames were paid for and never seen.
- **The per-gesture event count of a real trackpad is unmeasured.** Synthetic `CGEvent` scrolls
  cannot reproduce momentum phases, which originate in the trackpad driver. It bounds the perceived
  *distance* of a flick, never the freeze; every tick provably moves exactly `scroll_lines` rows.
- **The bar is shown whenever scrolling is possible**, and the free space is the message: the thumb
  never fills the track, so space above it means "more above" and space below means "more below".
  It is capped at four fifths of the track, since a thumb that nearly fills its track reads as
  "nothing to scroll". Only two panes show no bar: one whose rows fit, and a single-cell track,
  which cannot seat a thumb and a gap at once. A pane **4 columns wide or narrower** also shows
  none — the badge gutter and the bar would leave the names nothing.
- **`--pick` from a home-sized root still cannot search** (`076`), and **a quit during a terminal
  handover is still swallowed** (`077`). Both are filed, neither is fixed.

# Verification

- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` — all green;
  **206 tests** (58 + 66 + 82).
- Hands-on, through real PTYs: scroll distance at `scroll-lines` 1/3/5/7/10 via flag, config, and
  live `ctl set`; out-of-range refused by flag and socket and clamped by config; press-only inert,
  press-and-release selecting, slide-off revoking, chevron press released on the name not toggling,
  each observed through `ctl get-path`; the scrollbar looked at on screen at the top, at the
  bottom, with `--no-scrollbar`, with a tree that fits, and in a 14-column pane.
- Latency re-measured after every subsequent change: flick 785 ms → **0–4 ms**, overscroll
  3 483 ms → **0–3 ms**.

# Session log

- Scoped and cut: `069`, `067`, `068`. Branch `sprint/017` cut from `main`.
- `069` reproduced with a new PTY wheel-feed harness (synthetic SGR wheel events, keypress-latency
  metric). Both symptoms measured: 785 ms frozen on a flick, 3 483 ms after overscrolling a
  9 156-row tree. The stated leading suspect — peek-loading — accounts for ~18 %; git for none; and
  a burst of 1 000 `Down` keypresses freezes identically, so the defect is the event loop, not the
  wheel. Root cause: one full `rows()` rebuild per event (twice per input event) with no
  coalescing and an unbounded queue. A throwaway spike measured the fix at 3–4 ms.
- Scope grew by one at the maintainer's request: `075-configurable-scroll-speed`, designed against
  the existing settings plumbing (range 1–10, default 3, error on the CLI, clamp in the config,
  error response over the socket).
- `067` and `068` designed, completing the design phase: `067` as ADR 0025 (press arms, release
  completes, same row and zone), `068` as an inert one-column indicator at the far right with the
  usual toggle surface.
- Design approved: ADRs 0024 and 0025 `Proposed → Accepted`; the four tasks `Draft → Designed`;
  the sprint `Designing → Implementing`. Design merge to `main`.
- `069` and `075` designs approved. Settled with them: the batch takes a ~8 ms time budget, the
  loop contract becomes **ADR 0024** (`Proposed`), `075` keeps the 1–10 range, and `069` is
  retitled "Input bursts freeze the pane" since the keyboard freezes identically.
- Independent review: no majors, ten minor findings, all fixed — including one queued event running
  behind a terminal hand-off, a `scroll-lines` above 255 discarding the whole config file, and a
  mid-batch status message that could never be drawn. A pre-existing quit-swallow the review found
  outside the diff became `077`. Closed out; gates green.
