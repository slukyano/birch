---
type: Task
title: Add the context menu
description: The right-click menu as the primary action surface - split out of the 0.5 bundle so it can land before the file ops it will eventually host.
status: Designed
priority: medium
---

Split out of `029-add-file-operations` (the phase-0.5 bundle), on the same reasoning that pulled
`028-add-copy-paths` out of it: the menu is the action surface the whole design leans on, and it is
useful before the four file operations exist. The design doc calls it "the primary surface for
everything else" — printable characters are permanently reserved for search, so the menu is not a
convenience, it is where actions live.

## The surface

[The design doc](../../docs/design.md) already specifies the menu:

```
Open
Open with…            (later)
──────────
New File…             ^N
Rename…               F2
Delete                ⌦
──────────
Copy Name
Copy Relative Path    ^⇧C
Copy Absolute Path
──────────
Reveal Root Here
```

- **Right-click opens it**, keyboard works inside it (arrows / Enter / Esc), and Esc dismissal
  slots into the layered back-out of [ADR 0012](../../docs/adr/0012-esc-backs-out.md).
- **Right-click below the tree** targets the root.
- **Compacted chains**: a segment-click scopes the menu to the clicked directory, not the tail.
- Every entry is a call into the shared action layer — the same one hotkeys, mouse, and the socket
  use. Menu-specific logic is a smell (an explicit architecture rule).

## What lands when

The menu can ship ahead of its contents. Entries whose action does not exist yet are either absent
or disabled-and-visible; picking one over the other is a design question, since a disabled row
advertises a roadmap and an absent row keeps the menu honest. Ordering with the neighbours:

| Task | Relationship |
|------|--------------|
| `028-add-copy-paths` | The three Copy entries; the copy primitive can land first, keyed only. |
| `029-add-file-operations` | New / Rename / Delete entries; inline editing is the harder half. |
| `034-add-open-with` | Adds one entry to an existing menu. |
| `073-hotkey-reference` | A "Keyboard Shortcuts" entry is one of the candidate surfaces there. |

## Open questions

- **Rendering**: an overlay widget drawn over the tree, clipped and flipped near the pane edges — a
  side pane is narrow, and a menu wider than the pane must still be readable.
- **`--pick` mode**: mutations are disabled by default there, which leaves a menu of Open / Copy /
  Reveal. Whether the menu appears at all in the picker is a product call.
- **Hover highlight** (SGR 1003 motion tracking) is bundled with the menu in the design doc's
  phase 0.5; it may belong here or in its own task.
- **Selection coupling**: whether right-click moves the selection to the clicked row or acts on it
  without moving — the click model ([ADR 0015](../../docs/adr/0015-click-selects-double-click-activates.md), and
  `067-select-on-mouse-up`) settles the left button only.

## Design

The menu is not a widget with a list of things it can do. It is a **projection of the action layer**
([ADR 0029](../../docs/adr/0029-the-menu-projects-the-action-layer.md)): actions declare themselves,
and the menu draws whichever ones are available for the current target. That is what makes
`AGENTS.md`'s "menu-specific logic is a smell" enforceable rather than aspirational, and it is why
this task is worth landing before the operations it will host.

### The action layer

`birch-core` gains an `action` module — no ratatui, so the crate boundary holds — carrying, per
action: an identity, a **label**, an optional **accelerator** to display, and an **availability**
predicate over `(targets, mode)`. `targets` is the selection from
[`074`](074-add-multi-selection.md), in tree order, so an action that is only meaningful for one row
and one that acts on many are the same shape.

`birch` gains one `perform(action, targets)` entry point. Hotkeys, the mouse, the menu, and the
socket all route through it; nothing reaches an operation another way.

### What the menu contains when this lands

**An action that does not exist is absent, not disabled.** A greyed row advertises a roadmap the
product has not shipped; an absent one keeps the menu honest and costs a user nothing. The menu
therefore *grows* as its contents land, with no menu change per task — each task adds an action, and
the projection picks it up.

| entry | source | present when `071` lands |
|---|---|---|
| Open | exists today (`NavEffect::Open`, `activate`) | **yes** |
| Reveal Root Here | exists today (`set_root`, behind `Verb::SetRoot`) | **yes** |
| Copy Name / Relative Path / Absolute Path | `028-add-copy-paths` | no |
| New File… / Rename… / Delete | `029-add-file-operations` | no |
| Open with… | `034-add-open-with` | no |

So this sprint ships a **two-entry menu** and the whole surface underneath it. That is the split the
sprint scope approved deliberately: the surface is the hard part, and each of sprint 021's tasks
then adds an action rather than designing a menu.

**No entry in this sprint is plural-aware** — Open acts on the cursor, as `Enter` does
([`074`](074-add-multi-selection.md)), and Reveal Root Here needs one directory. The availability
predicate already takes the whole target set, so `Copy Paths` and a multi-target `Delete` plug in
without redesign.

### Opening, targeting, and the selection

**Right-click completes on release**, on the row the press started on — the left button's rule
([ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md)) with no exception carved for the
right one. Sliding off before releasing abandons it.

Targeting follows the rule every file manager uses, and it is the reason right-click is a *selecting*
gesture only when it has to be:

| where the right-click lands | what happens |
|---|---|
| a row **inside** the current selection | the selection is untouched; the menu acts on all of it |
| a row **outside** the current selection | the selection collapses to that row, then the menu opens |
| **below the last row** | the selection collapses to the root row, then the menu opens |
| a **dimmed** row | nothing — a dimmed row is inert by every route ([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)) |

**On a compacted chain the target is the tail**, not the clicked segment. `Row::path` is the tail
everywhere in birch — selection, `get-path`, and the copy variants all resolve to it — and
[`074`](074-add-multi-selection.md) has just settled that a chain row contributes one path. A menu
that scoped to a segment would be the only surface in birch that reads a chain differently, and it
would need `hit_test` to resolve a column to a segment, which it does not do today
(`crates/birch-tui/src/render.rs:634` returns `(row index, on_chevron)`). Segment reach stays what it
already is: `→` splits the chain into member rows
([ADR 0014](../../docs/adr/0014-chains-split-on-demand.md)) and each member is then a row like any
other. **This narrows `docs/design.md:50` and `:110`, which promise segment-scoped menus**; both
lines are corrected to name the split as the reach.

### Inside the menu

| input | effect |
|---|---|
| `↑` / `↓` | move between entries, skipping separators; stops at the ends |
| `Home` / `End` | first / last entry — the same keys `081` binds in the tree |
| `Enter` | invoke the highlighted entry and close |
| `Esc` | close, changing nothing |
| left click on an entry | invoke and close (on release, as everywhere) |
| left click outside the menu | close, changing nothing — the click is consumed, not passed through |
| any printable character | **close the menu and type into search** |

The last row is the one that could have gone another way. Menus conventionally offer type-ahead, but
[ADR 0008](../../docs/adr/0008-q-types-into-search.md) states without exception that printable
characters are search input, and a menu that quietly redefines the keyboard is how a modal language
starts in a product whose identity refuses modes. Typing is always search; the accelerators drawn
beside the entries are what teach the real keys.

**`Esc` gains a layer.** [ADR 0012](../../docs/adr/0012-esc-backs-out.md)'s back-out order becomes
**menu → search → quit**: with a menu open over a live search, the first `Esc` closes the menu, the
second clears the search, the third quits.

### Hover, and the motion tracking it needs

A menu that does not light the entry under the pointer cannot be used with a mouse. Motion tracking
(SGR 1003) is therefore enabled **when the menu opens and disabled when it closes**, so the tree
never pays for the event volume. Hover in the tree itself is a different feature with a different
cost profile and is filed as `090-hover-highlight-in-the-tree`.

### Geometry

The menu is an overlay drawn after the tree into the same `Rect`; the tree beneath it is not
re-laid-out, and `hit_test` is unchanged.

- **Width** = the widest `label + gap + accelerator`, plus a one-column pad each side, clamped to the
  pane. When the pane is narrower, **accelerators are dropped first** — they are redundant hints —
  and only then are labels ellipsised with `…`.
- **Horizontally** the menu's left edge sits at the click column, **shifted left** just enough to fit
  when it would overflow the right edge. Shifting rather than flipping keeps it adjacent to the
  pointer, which matters most in a narrow pane where a flip would move it across the whole width.
- **Vertically** it grows down from the click row, and **flips to grow upward** when it would
  overflow the bottom.
- **Taller than the pane**: clamped to the pane height, and the highlighted entry is scrolled into
  view — the same follow rule the tree uses. Not reachable with two entries; designed now because
  the menu is specified to reach eleven.
- **Separators** are a rule row, never highlighted, skipped by `↑`/`↓`.

### `--pick`

**The menu appears in the picker too.** The picker is the same tree rendered the same way, and
ADR 0023 deliberately removed the last place the two modes diverged; a surface that exists in one
mode and not the other is a second mental model to learn. Mutating actions are unavailable there by
their availability predicate — which today excludes nothing, since none exist yet — and `Open` in a
picker means *pick*, routed through the same action.

### Public-surface delta

| surface | change |
|---|---|
| CLI flags | none |
| config keys | none |
| environment variables | none |
| socket / protocol | none — no verb, and no mutation verb |
| on-disk paths / formats | none |
| key bindings | none new in the tree; `↑` `↓` `Home` `End` `Enter` `Esc` gain meanings **inside** the menu, and `Esc` gains a layer |
| mouse | right-click opens the menu (previously unmapped) |
| library API | `birch_core::action` is new — an `Action` identity, its label, accelerator, and availability predicate |

### Implementation

- **`crates/birch-core/src/action.rs`** — the `Action` enum, metadata, and availability.
- **`crates/birch-tui/src/menu.rs`** — the menu model (entries, highlight, scroll) and its geometry:
  width, shift, flip, clamp.
- **`crates/birch-tui/src/render.rs`** — painting the overlay; a `menu_hit_test` for entry rows.
- **`crates/birch-tui/src/input.rs`** — right button `Press`/`Release`, and motion events while the
  menu is open.
- **`crates/birch/src/app.rs`** — menu open/closed state, the input routing split (menu first, tree
  second), `perform(action, targets)`, the `Esc` layer, and enabling/disabling SGR 1003.
- **`docs/design.md`** — the two segment-scoping lines corrected; the menu block updated to what
  ships; the `Esc` row extended.

### Tests and verification

Unit: geometry (shift at the right edge, flip at the bottom, clamp-and-scroll in a short pane,
accelerator drop then ellipsis in a narrow one); targeting (inside the selection leaves it, outside
collapses it, below the rows selects the root, a dim row does nothing); `↑`/`↓` skipping separators;
a printable closing the menu and reaching search; `Esc` ordering with a search active; availability
filtering an absent action out of the projection. Hands-on: `vhs` frames of the menu at the pane's
right edge, at its bottom, and in a pane too short to hold it — a visual change, so frames are
required, not optional.
