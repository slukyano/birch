---
type: Task
title: Open with names the adapter instead of the thing it opens
description: The menu row reads "Open with birch-cmux" because the open command's program is the adapter shim; the open command needs a display name adapters can set.
status: Done
priority: medium
---

The `Open` row names the program the open command runs
(`crates/birch-core/src/action.rs:189`), taken from the command's argv[0] basename
(`crates/birch-core/src/open_cmd.rs:120-124`):

```rust
pub fn program(&self) -> &str {
    self.argv.first().map_or("", |arg| {
        arg.rsplit(std::path::MAIN_SEPARATOR).next().unwrap_or(arg)
    })
}
```

That comment says the row exists so it "names what will actually happen rather than a category".
Under a host adapter it names the shim instead. All three adapters bake themselves into
`--open-cmd` — `contrib/birch-cmux:122`, `contrib/birch-tmux:65`, `contrib/birch-herdr:57` — so the
menu reads **"Open with birch-cmux"**, "Open with birch-tmux", "Open with birch-herdr". None of
those is a program a person opens files with; each is the glue that routes the open to the host.

The heuristic is right for the case it was written for. `nvim +{line} {}` should read `nvim`, not
the whole template. It has no way to know that argv[0] is a courier.

## Fix

Give the open command an optional **display name**, set by whoever knows what the open really does,
and prefer it over the argv[0] basename when present. `birch-cmux` sets `cmux`. Absent a display
name nothing changes — `nvim` still reads `nvim`.

`birch-tmux` and `birch-herdr` set theirs in [`097`](../tasks/097-tmux-herdr-set-their-display-name.md), one
line each, alongside the live session that observes them. This task is the mechanism plus the host
the bug was reported against.

## Open for design

- **Where the name arrives.** A launch flag beside `--open-cmd`, the runtime setter
  [`083`](083-birch-cmux-does-three-things.md) introduces, or both. The two tasks want one answer:
  a command set at runtime that cannot carry its own name would reintroduce this bug through the
  new surface.
- **What constrains it.** It is drawn in a menu row, so it needs a length bound and a rule for
  control characters and newlines — a value that arrives over the socket is not a value birch chose.
- **Whether the fallback should also improve.** A `birch-` prefix is a strong hint that argv[0] is
  an adapter; stripping it would read `cmux` with no configuration at all. Against it: it is a
  guess about a naming convention, and it would silently rename a program genuinely called
  `birch-something`.

## Note on overlap

[`083`](083-birch-cmux-does-three-things.md) replaces `birch-cmux`'s `--open-cmd` shim with
`cmux open {} --window <win>` set at runtime, whose argv[0] basename is already `cmux`. So the
**reported** instance of this bug is fixed incidentally by that reshape. This task is still needed:
`birch-tmux` and `birch-herdr` remain shims by design, and an incidental fix is one refactor away
from regressing. The display name makes the correct row a decision rather than a side effect.

## Design

### The mechanism

`OpenCmd` gains an optional display name. `program()` keeps doing exactly what it does — the argv[0]
basename, which is right for `nvim +{line} {}` — and a new accessor prefers the name when one is
set:

```rust
pub fn display_name(&self) -> &str {
    self.name.as_deref().unwrap_or_else(|| self.program())
}
```

`ActionContext.open_program` is fed from `display_name()` instead of `program()`
(`crates/birch/src/app.rs`, `action_context`). Nothing else in the label path changes: the row still
reads `Open with <thing>`, `Label::naming` still dims the name, and a directory row still says
`Expand`/`Collapse`.

### Where the name arrives

Both surfaces, one answer, as the sprint ordering requires:

| Surface | Shape | Who uses it |
|---|---|---|
| `--open-name <text>` | Launch flag beside `--open-cmd` | `birch-cmux` at launch, and `birch-tmux`/`birch-herdr` in [`097`](../tasks/097-tmux-herdr-set-their-display-name.md), which bake and never set |
| `name` field on `set-open-cmd` | Optional; absent leaves the current name unchanged | ADR 0035's setter, so a command replaced at runtime cannot lose or contradict its label |

Baking alone would leave the runtime setter able to reintroduce the bug it was built beside;
the setter alone would leave `birch-tmux` and `birch-herdr` with no way to fix their label at all,
since they have no watcher to call anything afterwards.

### Bounds

The name is drawn in a menu row and one of its two paths arrives over a socket, so it is bounded and
cleaned where it enters: control characters and newlines stripped, then truncated to **32
characters**. `Open with ` plus 32 already exceeds a narrow dock pane, so the cap costs nothing real
and removes the question of what a hostile or careless value can do to the frame. The same cleaning
applies to the launch flag — not because a person needs protecting from their own command line, but
because two paths with different rules is how one of them ends up wrong.

Rejected, and listed in the task as open: **stripping a `birch-` prefix from the basename**. It
would fix all three adapters with no configuration at all, and it is a guess about a naming
convention — it would also rename a program genuinely called `birch-something`, silently, with no
way to ask for the real name back.

### Public-surface delta

Both rows above. The socket field is additive per ADR 0011; the launch flag is new and optional, and
an absent name changes nothing — `nvim` still reads `nvim`.

### Tests

- `display_name()` returns the name when set and the basename when not.
- The label for a set name reads `Open with cmux`; unset, it still reads `Open with nvim`.
- A name of control characters, newlines, and 200 printable characters yields one clean row of at
  most 32.
- `set-open-cmd` without `name` leaves an existing display name in place.
- Rendering is read rather than asserted for the visual half: the row is drawn, so the menu is
  captured from a real birch and the line inspected, per the hands-on gate.

### Scope

The mechanism, and `birch-cmux` setting `cmux`. `birch-tmux` and `birch-herdr` set theirs in
[`097`](../tasks/097-tmux-herdr-set-their-display-name.md), in the sprint that runs those hosts.
