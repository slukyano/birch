---
type: ADR
title: The control client is a `birch ctl` subcommand, not a separate binary
status: Accepted
sprint: sprint-013
---

# Context

The control-socket client shipped as a separate binary and crate, `birch-ctl`, following the
systemd `-ctl` convention (ADRs [0010](0010-socket-addressing-and-lifecycle.md) /
[0011](0011-ndjson-protocol.md)). birch is a foreground TUI, not a background daemon, and the
client is tiny — one NDJSON round-trip. Adopting cargo-dist
([ADR 0018](0018-release-via-cargo-dist.md)) for releases surfaced the cost: cargo-dist is
per-package, so two crates meant two Homebrew formulae (`birch`, `birch-ctl`), and the adapters
need the client on `PATH` for reverse-reveal — so a single `brew install birch` had to pull it
too.

# Decision

Fold the control client into the `birch` binary as a **`birch ctl <verb>`** subcommand — the
tmux model, where one binary is also its own client. Remove the `birch-ctl` crate. `birch`
launches the tree by default; when the first argument is `ctl`, it runs the control client
instead (hand-dispatched, because the launch form's optional `[DIR]` positional cannot be
cleanly disambiguated from a clap subcommand). The socket protocol and addressing (ADRs
0010/0011) are unchanged — only the client's invocation moves from `birch-ctl <verb>` to
`birch ctl <verb>`.

# Consequences

- One binary, one crate, **one Homebrew formula** — cargo-dist's per-package model now yields
  exactly one app.
- Less surface to install and document; the adapters call `birch ctl … reveal`, and the
  `$BIRCH_CTL` override is gone (the binary is named by `$BIRCH`).
- The `birch` binary carries the small client module and its clap subtree; negligible.

# Alternatives considered

- **Two formulae** (`birch`, `birch-ctl`), optionally with `birch` depending on `birch-ctl` —
  more moving parts and a cross-formula dependency for one small tool.
- **Keep `birch-ctl` as a second binary of one package** — a package can hold multiple binaries,
  but there is no reason to keep a thin client as its own binary once it can be a subcommand.
