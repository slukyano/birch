---
type: Task
title: Move files and folders by dragging
description: Drag a row onto a directory to move it - requested by the maintainer; currently forbidden by the scope fence, so it needs a fence amendment before design.
status: Done
priority: medium
---

Maintainer request: drag a row onto a directory row and drop it there to move the file or folder,
the way Finder and the IDE explorers do.

## Scope-fence conflict (decide first)

[The design doc](../../docs/design.md) forbids this three times over, and the fence is binding:

- **"No move (see below)"** (File operations) — no longer a permanent scope prohibition since
  [ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md), but still the section's own
  stated decision, argued from rename-to-a-path covering the 80%;
- **"No drag-and-drop"** (the mouse section);
- **"Exactly four: rename, delete, new file, new dir. No move (see below), no copy"** (file
  operations), where the rationale is that *rename-to-a-path* already covers move: editing a row
  inline to `../other/name.ts` performs the move, called "the 80% of move without the complexity
  cliff".

So this task cannot be designed as written until the fence moves. Amending it is a maintainer
decision and warrants an ADR — recording what the fence now says, and why the 80% path stopped
being enough (dragging is the discoverable gesture; inline rename-to-path is invisible to anyone
who has not read the docs).

## What the feature is

- Press on a row, move past a threshold, drop on a directory row → the path moves into it.
- Drop on a file row targets its parent directory; drop below the tree targets the root.
- Visible feedback: the dragged row's name follows the cursor (or the row lifts/dims), the drop
  target directory highlights, and an illegal drop reads as illegal before the button is released.

## The hard parts

- **Drag vs. click.** Under mouse capture a drag *is* press → motion → release, which is exactly
  the click model's shape. [ADR 0015](../../docs/adr/0015-click-selects-double-click-activates.md) acts on
  button-down and `067-select-on-mouse-up` proposes moving that to release; whichever wins, a drag
  threshold (cells moved, or a hold delay) has to separate "clicked" from "started dragging"
  without making ordinary clicks feel sticky.
- **Shift-drag is already taken.** Mouse capture disables native text selection, and the documented
  mitigation is that Shift-drag passes through to the terminal. That escape hatch must survive.
- **Auto-scroll and spring-loading.** A move usually crosses more tree than one screen shows:
  dragging to the pane edge should scroll, and hovering a collapsed directory should spring it open
  (Finder's behaviour) — which collides with **"never auto-expand"**, a standing rule motivated by
  ignored directories. A hover-expand is human-initiated, but the rule deserves an explicit carve-out
  rather than a quiet exception.
- **Illegal and colliding drops.** A directory into its own descendant; a drop where the name
  already exists (refuse, or offer a rename — no overwrite either way); a read-only destination;
  a compacted chain, where the drop target is a *segment*, not the visible row.
- **Undo.** There is deliberately no op-history stack, and trash cannot help a move. The reverse of
  a move is another move, which means the status line has to say clearly what happened and where it
  went.
- **Cross-instance and cross-root drags are not in this.** With `026-add-multiple-roots`, dragging
  between sibling roots is the same in-process gesture; dragging *between terminal panes* is not
  something a TUI can receive, and should be stated as a non-goal so it is not read as missing.

## Related

- `029-add-file-operations` — the op layer this rides on (watcher-event ownership during mutation,
  no flicker, no selection jumps).
- `071-add-context-menu` — a "Move to…" menu entry is the keyboard-reachable twin of the gesture,
  and may be the cheaper half of the same product goal.
- `074-add-multi-selection` — dragging a multi-selection is the obvious next ask; the two share a
  fence amendment.

## Design

[ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md) turned "no move" from a permanent
prohibition into the File operations section's own decision, so this needs an argument rather than an
amendment. The argument is discoverability, and it is recorded in
[ADR 0032](../../docs/adr/0032-move-by-dragging.md): rename-to-a-path already performs a move and
**nobody discovers it**, because nothing on screen suggests that editing a name accepts `../`.
Dragging is the gesture people try first, and since `074` a selection can hold several rows, which
rename-to-a-path cannot move at all.

### A drag is a click that left its row

[ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md) already arms on press and completes
on release on the row the press started on, and already abandons anything else. A drag slots into
that model with **no threshold to tune**: the drag begins the moment the pointer leaves the row the
press started on.

No timer, no distance heuristic, no stickiness. A click that wobbles inside its own row is still a
click, which is what ADR 0025 already guarantees. And the events arrive with no new terminal mode:
crossterm's mouse capture already enables button-event tracking, so `MouseEventKind::Drag` is
delivered while a button is held — unlike the motion tracking the menu needs, which birch enables for
the menu's lifetime alone.

**`Shift`-drag remains the terminal's**, untouched, as it has been throughout: birch never claims
`Shift` from the mouse.

### What is dragged, and where it can land

**The dragged set follows the menu's targeting rule.** A press on a row inside the selection drags
the whole selection; a press outside it collapses the selection to that row and drags only that. One
rule, already established, already understood.

| the pointer is over | the drop target |
|---|---|
| a directory row | that directory |
| a file row | its parent directory |
| below the last row | the root |
| a dimmed row | nothing — dimmed is inert ([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)) |
| a compacted chain | the chain's **tail**, like every other surface |

**Refused before the button is released**, with the target drawn as not-a-target and the status line
saying why: a directory into its own descendant; a destination where the name already exists; the
source's own parent, which would be a no-op. A refusal is visible during the drag rather than
discovered after it.

**A collision refuses; it does not overwrite, and it does not silently rename.** `029`'s inline editor
makes "offer a new name" possible later, and this design deliberately does not take it — a drop that
sometimes opens a text field is a surprising drop.

### Spring-loading is not the carve-out the task expected

The task assumed hovering a collapsed directory open would collide with a standing "never
auto-expand" rule. Reading the rule, it does not: `docs/design.md` scopes it to **ignored**
directories — *"Ignored dirs (e.g. `node_modules`) are never auto-expanded, never searched, and never
recursively watched"* — and search already auto-expands to reveal matches. So spring-loading a normal
directory needs no exception at all.

**Hovering a collapsed directory during a drag expands it; an ignored directory stays shut.** An
ignored directory remains a legal *drop target* — dropping into `node_modules` is odd but not
birch's business to forbid — it simply does not open itself to show you.

### Auto-scroll, and its honest limit

Dragging into the top or bottom row scrolls the viewport by a row per motion event. **Holding the
pointer still at the edge does not keep scrolling**, because motion events only arrive when the
pointer moves; continuing the drag continues the scroll.

Timer-driven scrolling would need a periodic tick in the event loop that exists for nothing else, and
this design does not add one for a case a small pointer movement already handles. It is named as a
limitation rather than engineered around, and a follow-up can add the tick if it grates.

### Feedback, in terminal terms

The task asked for the dragged row to follow the cursor. **It does not** — a floating overlay tracking
a pointer is a lot of machinery for a 30-column pane. Instead:

- **the drop target directory is highlighted**, distinctly from the selection, so what will receive
  the drop is never ambiguous;
- **the status line names the operation** — *"Moving 3 items → src/utils"* — and, on a refused
  target, why it is refused;
- **an illegal target simply does not highlight**, so the absence of feedback is itself the answer
  before release.

### After the move

The op layer from [`029`](029-add-file-operations.md) performs it and emits the deltas; the watcher's
snapshot converges as it does for every other mutation. **There is no undo** — there is no op-history
stack, and trash cannot help a move — so the status line reports what moved and where, because
nothing in birch can show it afterwards. The reverse of a move is another move.

**Dragging is unavailable in `--pick`**, with every other mutation.

**Cross-instance and cross-pane dragging are explicit non-goals.** Dragging between two birch panes
is not something a TUI can receive; with `026-add-multiple-roots` a drag between sibling roots would
be the same in-process gesture and needs nothing new here.

### Public-surface delta

| surface | change |
|---|---|
| mouse | **drag with the left button moves the dragged set into a directory** |
| key bindings · CLI flags · config keys · environment variables | none |
| socket / protocol | none — mutations stay human-initiated |
| on-disk | birch moves paths under the root |
| library API | `birch_core::ops` gains a move; no new `Action` variant, since a drag is a gesture rather than a menu entry |

### Implementation

- **`crates/birch-tui/src/input.rs`** — `MouseEventKind::Drag(Left)` → an `InputAction::Drag`.
- **`crates/birch/src/app.rs`** — drag state (the set, the origin row, the current target), promotion
  from an armed press once the pointer leaves its row, spring-load on hover, edge scrolling, and the
  drop through `birch_core::ops`.
- **`crates/birch-tui/src/render.rs`** — the drop-target highlight.
- **`docs/design.md`** — the File operations section records that move is available by dragging, and
  the mouse section describes the gesture.

### Tests and verification

Unit: a press that stays in its row is still a click and a press that leaves it becomes a drag; target
resolution for each row kind including a compacted chain and below-the-rows; every refusal — into a
descendant, onto a colliding name, onto the source's own parent, onto a dimmed row; the dragged set
following the selection rule; spring-load opening a normal directory and refusing an ignored one.
Hands-on: perform a real drag in a scratch directory and observe the filesystem, and record `vhs`
frames of a drag in flight and of a refused target, both being visual states that did not exist
before.
