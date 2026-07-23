---
type: ADR
title: Socket addressing, rendezvous, and lifecycle decisions
status: Accepted
sprint: sprint-004
---

# Context

The design doc fixes the socket scheme (`$XDG_RUNTIME_DIR/birch/<pid>.sock`, a
`by-root/<root-hash>.sock` symlink, `--socket` host rendezvous, SIGHUP/SIGTERM graceful
exit) and leaves three questions open: the base dir on platforms without
`XDG_RUNTIME_DIR` (macOS — the flagship dev platform), whether the by-root symlink's
most-recent-wins is enough for multiple instances, and whether `set-root` may point above
the original root.

# Decision

- **Base dir**: `$XDG_RUNTIME_DIR/birch` when set; otherwise
  `<std tmp dir>/birch-<uid>`. Created `0700`; if it exists with wrong ownership or
  permissions, the socket server refuses to start (filesystem permissions are the auth
  model, so the model must actually hold).
- **Root hash**: the same FNV-1a-64 scheme persistence uses — one root, one stable hash,
  two consumers.
- **Most-recent-wins stands**: the by-root symlink points at the newest instance for that
  root; `birch-ctl` resolves by walking up from cwd. Instance listing is not needed for
  any current user story — a host that runs several instances uses `--socket` and knows
  its own paths. Revisit only with a concrete story.
- **`set-root` may point anywhere** (any readable dir, canonicalized). The verb is
  explicit, human- or host-initiated intent; restricting it to descendants would break
  the "jump to a sibling project" case for zero safety gain — the socket caller already
  has filesystem access. The by-root symlink is re-pointed on re-root.
- **Lifecycle**: SIGHUP and SIGTERM trigger the normal quit path (state saved, terminal
  restored, socket and symlink unlinked) via the `signal-hook` crate. Stale sockets from
  crashed instances are unlinked on connect-refused when re-binding.

# Consequences

- macOS and Linux behave identically from the adapter's point of view; only the base dir
  differs.
- A crashed instance leaves a dangling symlink; the next instance on that root repairs
  it, and `birch-ctl` treats connect-refused as "no instance".
