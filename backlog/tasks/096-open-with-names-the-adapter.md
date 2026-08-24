---
type: Task
title: Open with names the adapter instead of the thing it opens
description: The menu row reads "Open with birch-cmux" because the open command's program is the adapter shim; the open command needs a display name adapters can set.
status: Draft
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

`birch-tmux` and `birch-herdr` set theirs in [`097`](097-tmux-herdr-set-their-display-name.md), one
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
