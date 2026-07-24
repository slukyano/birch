---
type: Task
title: Add the control socket and birch-ctl
description: Unix socket per instance, NDJSON protocol, closed verb set, --socket rendezvous, thin birch-ctl client.
status: Done
priority: medium
blocked_by:
- 002-build-core-tree-view
---

Phase 0.4 (partial) of [the design doc](../../docs/design.md): socket per instance under
`$XDG_RUNTIME_DIR/birch/`, per-root symlink resolution, host-dictated `--socket` rendezvous,
newline-delimited JSON with additive-only evolution, the closed verb set (`reveal`,
`get-path`, `get-root`, `set`, `set-root`, `open`, `quit` — no mutation verbs), clean
SIGHUP/SIGTERM exit. Socket dir `0700`; filesystem permissions are the auth model.

Design-phase open questions (from the design doc): is `set-root` above the original root
allowed, and is the by-root "most recent instance" symlink enough for multi-instance, or
does `birch-ctl` need instance listing? Both resolved in
[ADR 0010](../../docs/adr/0010-socket-addressing-and-lifecycle.md): `set-root` goes anywhere;
most-recent-wins stands.

## Design

Addressing/lifecycle per [ADR 0010](../../docs/adr/0010-socket-addressing-and-lifecycle.md);
wire protocol per [ADR 0011](../../docs/adr/0011-ndjson-protocol.md).

**birch-core** gains a `protocol` module: serde `Request`/`Response` types (+ verb and
setting enums), shared by server and client so shapes cannot drift; `socket_dir()`,
`instance_socket(pid)`, `by_root_link(root)` path helpers reusing persistence's FNV hash.
Pure; unit-tested serialization both ways plus unknown-field tolerance.

**birch** (server side):

- `--socket <path>` binds exactly there (host rendezvous, no symlink); default addressing
  binds `<dir>/<pid>.sock` and re-points `by-root/<root-hash>.sock`.
- A listener thread accepts connections; per connection, a small thread reads NDJSON
  lines and sends `AppEvent::Ctl { request, reply }` (a bounded sync channel carries the
  response back). The app loop stays single-threaded; verbs execute exactly like local
  actions (the action layer is shared — reveal is the search primitive, open is the open
  primitive, quit is the quit path).
- `set` flips `Settings` fields at runtime; `hidden` also rebuilds the search index with
  the new flag (the rebuild command now carries it), `files-first` re-sorts loaded dirs
  (new `Tree::set_files_first`). Mouse capture stays a startup flag — toggling terminal
  capture live is not in ADR 0011's setting set.
- `set-root` rebuilds tree/view/watches/index/git for the new root (state saved for the
  old root first, loaded for the new one) and re-points the by-root symlink.
- SIGHUP/SIGTERM (via `signal-hook`) feed a quit event: state saves, terminal restores,
  socket + symlink unlink. Stale sockets are unlinked and re-bound at startup.
- Sockets are never created in `--pick` mode (a transient picker is not a host surface).

**birch-ctl** (client): `birch-ctl [--socket <path>] <verb> [args]` — resolves the socket
via `--socket`, else `$BIRCH_SOCKET`, else walking up from cwd over by-root hashes;
sends one request line, prints `data` (if any) on ok, prints the error to stderr and
exits 1 otherwise; exits 2 when no instance is found. `get-path --name|--rel|--abs` maps
to the `form` field.

**Tests**: protocol round-trips; an end-to-end test binding a real socket with a stub
app loop answering verbs; birch-ctl's socket resolution (walk-up) against a temp tree;
setting parsing (`toggle`, `on`/`off`).
