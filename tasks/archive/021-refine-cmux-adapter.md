---
type: Task
title: Refine the cmux adapter after first live use
description: No focus stealing, side selection, dedicated preview pane (master → preview → tree), clean socket path, mise dev PATH.
status: Done
priority: high
---

First live-use feedback on birch-cmux, direction from the maintainer: double `//` in
the socket path; toggling must not steal focus; the tree side must be selectable;
previews belong in a dedicated pane with the default layout master pane → preview
pane → tree; and in-repo commands should just work without `BIRCH`/`contrib/`
prefixes.

## Design

- **Socket path**: strip the trailing slash macOS leaves on `$TMPDIR` before
  composing the state dir. All three adapters share the pattern; fix all three.
- **Focus**: `new-split --focus false` explicitly, and the spawn path ends by
  focusing the invoking surface again — the end state is deterministic: focus stays
  where the user was. (`open` on an *existing* tree still focuses it; that verb is an
  explicit "go to the tree".)
- **Side**: `BIRCH_CMUX_SIDE=left|right`, default `right` per the maintainer's
  default layout (master → preview → tree). An env var over a flag: zero parsing,
  and it sits naturally next to `BIRCH` in a keybinding or shell profile.
- **Preview pane**: `--open-cmd` becomes `birch-cmux preview <side> <master-surface>
  {}` — an internal verb like birch-tmux's `open-in-main`. It targets a dedicated
  preview pane between master and tree: reuse the pane recorded in a state file when
  still alive, else split it off the master surface (baked at spawn time) in the
  tree-side direction — carving from the master rather than the narrowed tree gives
  the preview half of the master's width. Then `cmux open <file> --pane <ref>
  --no-focus`. The bootstrap terminal surface is closed after the first open, and
  older preview tabs in that pane are closed on each open — single-preview
  semantics; terminal surfaces in the pane are never touched. Markdown/HTML get
  cmux's rich viewers, everything else a file preview tab. (Implementation deviation
  from the first draft, which split from the tree toward the master: same geometry,
  better widths.)
- **Socket stays** (maintainer question): the socket serves only inbound control —
  reverse-reveal and scripting (`birch-ctl reveal` from an agent or editor); the
  forward open path never uses it. It is cheap and it is the documented adapter
  promise, so it stays until the maintainer decides otherwise.
- **mise**: `[env] _.path = ["{{config_root}}/contrib", "{{config_root}}/target/debug"]`
  in `mise.toml` — inside the repo `birch`, `birch-ctl`, and the adapters resolve
  with no prefixes. The adapter also resolves `${BIRCH:-birch}` to an absolute path
  at spawn, so the split's shell needs no mise context of its own.
- **Testing convention** (new, recorded in AGENTS.md): never live-debug in the
  maintainer's own cmux instance — launch a separate instance and drive it via
  `CMUX_SOCKET_PATH`.
