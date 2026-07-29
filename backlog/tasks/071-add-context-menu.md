---
type: Task
title: Add the context menu
description: The right-click menu as the primary action surface - split out of the 0.5 bundle so it can land before the file ops it will eventually host.
status: Draft
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
