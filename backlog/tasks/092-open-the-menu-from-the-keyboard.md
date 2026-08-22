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
