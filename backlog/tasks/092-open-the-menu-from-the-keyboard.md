---
type: Task
title: Open the context menu from the keyboard
description: The menu has exactly one trigger - the right button - so a host that swallows right-click makes it unreachable, and there is no keyboard way in at all.
status: Draft
priority: medium
---

[`071-add-context-menu`](../archive/071-add-context-menu.md) bound the menu to the **right button
only**. The keyboard works fully *inside* it — arrows step entries, `Home`/`End` reach the ends,
`Enter` invokes, `Esc` closes — but the act of *opening* it has one trigger and no alternative.

That is a real gap rather than an oversight to shrug at, and it gets more expensive fast: sprint 021
fills the menu with Copy Name / Relative Path / Absolute Path, New File, Rename, and Delete. A menu
that some users cannot open is a menu that hides most of what birch can do — and printable
characters are permanently reserved for search
([ADR 0008](../../docs/adr/0008-q-types-into-search.md)), so the menu is where actions live by
design, not by preference.

It bites hardest in a pane host. Where the host consumes the right button before it reaches birch —
its own context menu, or a `right-click-action`-style setting on an embedded terminal — the menu is
unreachable in that host entirely, however well it works in a plain terminal.

## What a design has to answer

- **Which key.** Printables are gone, so the candidates are modified or non-printable. What is
  known about each, in the legacy encoding birch decodes with (see
  [`091`](091-kitty-keyboard-protocol.md)):
  - **`Ctrl-Space`** arrives as `NUL` and reaches crossterm as `Char(' ')` with `CONTROL` — viable,
    and distinguishable from everything birch binds.
  - **`Shift-F10`** is the Windows/Linux convention for "context menu". Function keys travel as CSI
    sequences that *do* encode modifiers, so unlike `Ctrl-Shift-C` this one survives the legacy
    encoding.
  - **The `Menu`/`Apps` key** (`KeyCode::Menu`) is the most semantically exact and the least
    available — Apple keyboards do not have one.
- **Where the menu appears** when no pointer opened it. At the cursor row is the obvious answer, and
  the existing geometry already shifts and flips to fit — but "at the cursor row" needs a column,
  and the row's indent is a better answer than column zero.
- **Whether it changes the selection.** The right button collapses the selection to the clicked row
  when the click lands outside it; a keyboard trigger acts on the selection that already exists and
  should move nothing.

## Diagnosis, which is a separate problem

When the menu does not open, two quite different things look identical from the outside: the host
swallowed the button, or the click landed somewhere birch deliberately ignores — the scrollbar
column, the status line, or a dimmed row. `074`'s design carried a probe that would have separated
them in seconds; it was dropped on the reasoning that birch handles whatever a terminal sends, which
was right about *behaviour* and wrong about *diagnosis*. The question has now come up twice. Whether
birch should ship a way to answer it — a probe in `contrib/`, or a debug mode that reports what
arrives — is worth settling alongside this, since a keyboard trigger removes the urgency but not the
question.

## Related

- `071-add-context-menu` — bound the single trigger this task adds an alternative to.
- `091-kitty-keyboard-protocol` — would widen the candidate keys considerably.
- `073-hotkey-reference` — would list whatever this binds.

## Design

The menu already exists, is already keyboard-navigable once open, and is already opened by a
function that takes a target row and a point. What is missing is a second way to call it.

### The key: `Shift-F10`, with the `Menu` key beside it

Both survive the legacy key encoding birch decodes with, which is the constraint that killed
`Ctrl-Shift-C` in [`028`](028-add-copy-paths.md):

| candidate | arrives? | verdict |
|---|---|---|
| **`Shift-F10`** | yes — function keys travel as CSI sequences (`CSI 21;2~`) that *do* carry a modifier parameter | **bound.** It is the Windows and Linux convention for "context menu", so it is the one a user is most likely to try. |
| **`Menu` / `Apps`** (`KeyCode::Menu`) | yes, where the keyboard has the key | **bound too.** Semantically exact and free — binding it costs one match arm, and it does nothing on keyboards that lack it. Apple keyboards lack it, which is why it cannot be the only binding. |
| `Ctrl-Space` | yes, as `NUL` → `Char(' ')` with `CONTROL` | **not bound.** It arrives, but it reads as nothing in particular, and `Ctrl`-modified keys are the scarce space [`091`](091-kitty-keyboard-protocol.md) would open up — spending one on a gesture that already has a conventional key is the wrong trade. |

Two bindings for one action is not a hedge here: they are the same convention on different
keyboards, and neither collides with anything birch binds.

### Where it opens, and what it must not disturb

**At the cursor row, indented to the row's own text.** The menu's geometry already shifts left at the
right edge, flips upward at the bottom, and clamps in a short pane
([ADR 0029](../../docs/adr/0029-the-menu-projects-the-action-layer.md)), so placing it needs only a
point — and the row's indent is a better column than zero, because a menu hanging off the far left
of a deep row reads as belonging to the tree rather than to the selection.

**The selection is not touched.** A right-click has to reconcile a *pointer* with a selection, which
is why it collapses to the clicked row when the click lands outside it. A keyboard trigger has no
such conflict: it acts on the selection that already exists, and moves nothing. That difference is
the whole reason `open_menu` already takes its target as a parameter rather than deriving it from a
hit test.

**Nothing selectable means no menu.** When a narrowing leaves no live row
([ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)) there is no target, so
`Menu::open` finds no available action and the key does nothing — the same outcome a right-click on
a dimmed row already has.

### What this does not fix

A host that swallows the right button still swallows it. This makes the menu *reachable* despite
that; it does not make right-click work, and it does not tell anyone which of the two is happening.
The diagnosis question stays open above, deliberately unbundled.

### Public-surface delta

| surface | change |
|---|---|
| key bindings | **`Shift-F10`** and the **`Menu`** key open the context menu at the cursor row |
| CLI flags · config keys · environment variables | none |
| socket / protocol | none |
| on-disk paths / formats | none |
| library API | none — `birch_core::action` is unchanged; this is a second caller of an existing surface |

### Implementation

- **`crates/birch-tui/src/input.rs`** — `InputAction::OpenMenu`, mapped from `KeyCode::F(10)` with
  `SHIFT` and from `KeyCode::Menu`.
- **`crates/birch/src/app.rs`** — one dispatch arm: resolve the cursor row to a point (its `y` from
  the viewport, its `x` from the row's indent), then call the existing `open_menu` with the cursor's
  path as the target, and enable motion tracking exactly as the mouse path does. The menu's own
  input handling is untouched.
- **`docs/design.md`** — the Keyboard table gains a row; the mouse section stops implying
  right-click is the only way in.
- **`README.md`** — the navigation paragraph names it, since a menu nobody can find is the problem
  this task exists to solve.

### Tests and verification

Unit: `map_event` for `Shift-F10` and `Menu`, and that plain `F10` is *not* bound; opening from the
keyboard leaves a multi-selection exactly as it was, where a right-click outside it would collapse
it; no menu opens when nothing is selectable. Hands-on: drive the binary, press the key, and confirm
from the render stream that the menu appears at the cursor row and that `Esc` still backs out of it
before the search.
