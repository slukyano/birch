---
type: Sprint
title: Pointer feel
status: Designing
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

No new sources, no new modes, no actions. New public surface: `075`'s scroll-speed setting (flag,
config key, `ctl set` key) and whatever toggle `068` needs.

# In-scope task ledger

- **`069-fix-wheel-scrolling`** — *bug, high, design-light but investigation-heavy.* Wheel
  scrolling "runs away sometimes" and overscrolling "makes it all freeze". `SCROLL_LINES` is
  already 3 and `scroll_by` already clamps, so the nominal step and the bounds are not the fault.
  The leading suspect is viewport-driven peek loading (`app::request_peeks` emits a
  `SourceCmd::Expand` per unloaded directory in view, every frame, and each arriving snapshot
  rebuilds the rows) — a burst that grows with how far the wheel travels. Reproduction comes first
  and needs a live terminal or a synthetic event feed; `vhs` cannot send wheel events.
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
  double-click is still two complete clicks. Amends or supersedes
  [ADR 0015](../../docs/adr/0015-click-selects-double-click-activates.md).
- **`068-add-scrollbar`** — *mid, medium.* A scroll indicator down the pane's edge, hidden when
  everything fits, with a way to turn it off. The right two columns are the badge gutter today
  (`render::BADGE_WIDTH`), so the layout must give it a column or share one — and whatever it takes,
  `hit_test` must account for. Pure indicator in this version; dragging the bar needs mouse-drag
  tracking birch has never had, and belongs with `072`.

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

- [ ] 069-fix-wheel-scrolling
- [ ] 075-configurable-scroll-speed
- [ ] 067-select-on-mouse-up
- [ ] 068-add-scrollbar

# Open questions

- **`069` — how the drain is bounded.** An unbounded drain of the queue can starve the screen under
  a continuous stream (the pane would stop repainting while a long flick is still arriving). A cap
  — a maximum batch size, or a frame budget after which the batch is drawn regardless — keeps the
  tree painting during the gesture. Recommendation: a time budget, since it degrades with machine
  speed rather than with event count.
- **`069` — whether the loop change warrants an ADR.** "One event, one frame" becoming "one batch,
  one frame" is a change to the app loop's contract, and every future input path inherits it.

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
