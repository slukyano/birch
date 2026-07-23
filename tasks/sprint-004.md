---
type: Sprint
title: Integration — control socket, birch-ctl, adapter and recipes
status: Done
branch: sprint/004
tasks:
- add-control-socket
- add-host-adapter-and-recipes
---

# Scope rationale

The last two MVP tasks (ADR 0001): the socket + `birch-ctl` make birch controllable from
outside, and the adapter/recipes make the persistent-sidebar story real. The design doc's
cut principle says this is the part that beats features. After this sprint the MVP is
complete.

# Checklist

- [x] add-control-socket
- [x] add-host-adapter-and-recipes

# Open questions

Both design-doc questions closed by ADR 0010 (`set-root` goes anywhere; most-recent-wins
symlink stands). Live herdr verification is `verify-herdr-integration`.

# Sprint summary

- **add-control-socket** (major): shared protocol module (ADR 0011 — versioned NDJSON,
  closed verb set, additive-only), addressing per ADR 0010 (`$XDG_RUNTIME_DIR`/tmp
  fallback with strict 0700 + ownership + no-symlink enforcement, by-root symlink on the
  persistence hash, stale-socket reclaim that never touches non-sockets), a listener that
  round-trips requests through the app loop so verbs share the action layer with keys and
  clicks, `set-root` rebinding tree/view/git/index/symlink, SIGHUP/SIGTERM through the
  normal quit path, and `birch-ctl` with walk-up socket resolution and 0/1/2 exit codes.
  PTY-verified end-to-end: reveal → get-path forms → set → set-root (symlink repointed;
  walk-up finds the new root) → quit unlinks; SIGTERM saves state and unlinks.
- **add-host-adapter-and-recipes** (mid): `contrib/birch-tmux` (verified live in a
  headless tmux session — spawn, host socket, reveal, open-in-main, toggle/SIGHUP
  cleanup, and a filename containing a quote and a space opening as one file) and
  `contrib/birch-herdr` (written against herdr's pane CLI; live verification is the
  `verify-herdr-integration` draft task, including the design doc's SGR passthrough
  check), plus `docs/integrations.md` — the four-point host promise, the adapter
  pattern, and no-adapter nvim/emacs/vscode recipes. ⚠️ Transformed: the design doc
  named birch-herdr as the exercised reference; the tmux adapter took that role because
  it can be driven headlessly, with herdr shipped alongside awaiting a live session.
- **Independent review**: two blockers — shell injection via filenames in both adapter
  scripts (fixed with strict POSIX quoting at every interpolation plus an argv-only
  open-cmd path) — and a security-focused set of should-fixes: socket-dir symlink
  attacks, non-socket unlink, by-root link lifecycle across set-root/exit, `set git`
  repo rediscovery, lexical root-boundary validation for reveal, private adapter state
  dirs. Verb and server test suites added (ctl_response is transport-free by design).
- **Known sharp edges**: one connection thread per client with a 1 MiB read cap but no
  idle timeout (same-uid auth model); `--socket` paths longer than the platform's
  `sun_path` limit (~104 bytes on macOS) fail to bind with a plain error; protocol `v`
  is echoed but version negotiation is trivially additive until v2 exists.

# Session log

- Sprint created; scope approved.
- Designs approved (ADRs 0010–0011 accepted); design merge.
- Implementation, PTY + live-tmux verification, security review fixes,
  sprint closed out. MVP complete (ADR 0001).
