---
type: Task
title: Add "Open with…" to the context menu
description: Choose an alternative open command for the selected node.
status: Draft
priority: low
blocked_by:
- 029-add-file-operations
---

Listed as "(later)" in the context-menu sketch of [the design doc](../../docs/design.md):
an "Open with…" entry offering alternative open commands. Needs a design for where the
alternatives come from (config?) before it can be scheduled; post-MVP.

## Design

The task was filed as *"Open with…"* with an open question about where a list of alternatives comes
from. **There is no list.** Maintainer decision: the menu offers exactly two ways to open, and
neither needs configuring.

```
Open nvim
Open system
```

### Two rows, no submenu, no config

- **`Open <command>`** is the existing `Open` action, relabelled to say what it will actually run.
  Today the row says `Open` and the user has to know what birch was started with; naming the command
  makes the menu describe the product instead of describing a category.
- **`Open system`** hands the path to the platform opener — `open` on macOS, `xdg-open` on Linux —
  detached, so it never takes the terminal. It is the one alternative that is universally useful and
  universally available, which is exactly why it needs no configuration to name it.

A configurable array of alternatives is **not built**. Nobody has asked for a third way to open a
file, and a config surface is far more expensive to change later than to add later.

### Where the label comes from

The command is already resolved at startup: `--open-cmd`, else the `open_cmd` config key, else
`$VISUAL`, else `$EDITOR`, else the platform opener (`docs/design.md`, Opening files). The row shows
the **program name** — the template's first word — rather than the whole template, because
`nvim +{line} {}` truncated into a narrow pane reads as noise while `nvim` identifies it exactly.
The menu's existing ellipsis handles a program name too long for the pane.

Two cases keep the plain `Open` label, because no command runs:

- **on a directory**, where `Open` toggles expansion;
- **in `--pick`**, where `Open` confirms the selection.

### When `Open system` is absent

**When the resolved open command already *is* the platform opener** — nothing configured, no
`$VISUAL`, no `$EDITOR` — the two rows would do the same thing, so the second is unavailable and
does not render (ADR 0029). The menu never shows two entries that are one entry.

**In `--pick`**, `Open system` is unavailable too: `Open` there means *confirm*, and a second row
that launches an application while the picker keeps waiting is a mode confusion, not a feature.

### Arity

**One target** — the cursor — for both rows, exactly as `Open` behaves today. Opening several files
means several processes, which is a different action and not one this task proposes.

### Public-surface delta

| surface | change |
|---|---|
| menu | `Open` is relabelled with the resolved program name on files; **`Open system`** is added |
| CLI flags · **config keys** · environment variables | **none** — the config array considered earlier is not built |
| socket / protocol | none |
| on-disk paths / formats | none |
| library API | `birch_core::action` gains `OpenSystem`; `Action::label` stops being a `&'static str` for `Open`, which now depends on its target |

### Implementation

- **`crates/birch-core/src/action.rs`** — the `OpenSystem` variant and its availability; `label`
  gains the context it needs to name the command, which is the first label that is not a constant.
- **`crates/birch-core/src/open_cmd.rs`** — the platform opener as a constructible `OpenCmd`
  (`open` / `xdg-open`, detached), which `default_cmd` already knows how to pick and can now expose.
- **`crates/birch/src/app.rs`** — a `perform` arm reusing `perform_open` unchanged, so detached
  spawning behaves exactly as it does for `--open-detached`.
- **`docs/design.md`** — the menu sketch's `Open with… (later)` row is replaced by the two rows.

### Tests and verification

Unit: the label showing the program name on a file, staying `Open` on a directory and in `--pick`;
`Open system` unavailable when the resolved command is already the platform opener, and in `--pick`;
the platform opener resolving per target OS. Hands-on: with `$EDITOR` set, open a file both ways from
the menu and confirm the first hands over the terminal while `Open system` does not.
