---
type: Sprint
title: cmux adapter live-use refinements
status: Done
branch: sprint/009
tasks:
- refine-cmux-adapter
---

# Scope rationale

Immediate live-use feedback on the sprint-008 cmux adapter, scope and design direction
given by the maintainer in chat: socket path cosmetics, focus discipline, tree side
selection, a dedicated preview pane (default layout master → preview → tree), and
frictionless in-repo commands via mise. One task; contrib + mise.toml + docs only, no
crate changes expected.

# Checklist

- [x] refine-cmux-adapter

# Open questions

None — the maintainer's socket question is answered in the task design (socket stays;
it serves reverse-reveal and scripting only).

# Sprint summary

- **refine-cmux-adapter** (mid): files open in a dedicated preview pane between
  master and tree (master | preview | tree; single preview, pane reused across
  toggles) via an internal `preview` verb; nothing steals focus (spawn ends by
  refocusing the invoker); `BIRCH_CMUX_SIDE=left|right` picks the tree side
  (default right), with the tree always splitting off the window-edge pane so
  toggling preserves ordering; `$TMPDIR` trailing-slash fix in all three adapters;
  `mise.toml` puts `contrib/` and `target/debug` on the in-repo PATH and the
  adapter bakes absolute paths so spawned shells need no mise context.
- **Independent review**: approve-with-nits; applied — tree-ref parsing anchored to
  node-type words (title tokens could get an unrelated surface closed), bootstrap
  split closed when the create-path open fails, survivor-ref guard on the cleanup
  loop, preview arg-count check, task design updated to the shipped interface.
- **Verification**: full live regression in a separate debug cmux window (both
  sides, preview reuse/replace, toggle ordering, focus discipline); cmux itself
  cannot run a second OS instance (single-instance guard), so isolation is a
  dedicated window driven by explicit IDs — recorded in AGENTS.md. Upstream quirk
  noted: `cmux close-window` does not SIGHUP pane processes the way
  `close-surface` does; birch exits cleanly on SIGHUP either way.

# Session log

- Sprint created from live-use feedback; designs are the maintainer's own
  direction. New testing convention adopted: separate cmux instance for debugging.
- Implemented, live-verified in a debug window, review fixes applied,
  closed out.
