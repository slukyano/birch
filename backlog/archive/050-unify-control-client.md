---
type: Task
title: Fold the control client into a `birch ctl` subcommand
description: Remove the separate birch-ctl binary/crate; control a running instance via `birch ctl <verb>` (tmux model). One binary, one formula.
status: Done
priority: high
---

The control client is a separate binary/crate (`birch-ctl`), following the systemd `-ctl`
convention. birch is a foreground TUI, not a background daemon, and its client is tiny — the
tmux model (one binary; a subcommand acts as the client) is the standard, simpler fit and
collapses release packaging to a single Homebrew formula. Fold `birch-ctl` into a `birch ctl`
subcommand and remove the separate crate.

## Design

**CLI.** Add `birch ctl [--socket <path>] <verb> [args]`, preserving `birch [DIR] [flags]`
unchanged. Because the launch form has an optional `DIR` positional, clap cannot cleanly
disambiguate a positional from a subcommand, so `main` **pre-dispatches**: if the first argument
is `ctl`, hand the rest to the control client; otherwise parse the launch `Cli` as today. The
client keeps its own clap parser (prog name `birch ctl`), so `birch ctl --help` and per-verb
help work. The launch `--help` gains a one-line pointer to `birch ctl`.

**Code move.** `crates/birch-ctl/src/main.rs` becomes `crates/birch/src/ctl_client.rs`, exposing
`pub fn run(rest: &[OsString]) -> ExitCode` (the former `main` body) plus its helpers
(`build_request`, `absolutize`, `resolve_by_walking_up`, `roundtrip`, `VerbCmd`, `SettingArg`).
It already depends only on `birch_core::protocol`, `clap`, and `serde_json` — all present in the
`birch` crate, so no new dependencies. Diagnostics change prefix `birch-ctl:` → `birch ctl:`.
Delete `crates/birch-ctl` and drop it from the workspace members. The server side
(`crates/birch/src/ctl.rs`) is unchanged.

**Adapters.** `contrib/birch-{cmux,tmux,herdr}` invoke/reference `birch-ctl … reveal`; rewrite to
`birch ctl … reveal` (birch-cmux's `${BIRCH_CTL:-birch-ctl}` default becomes `birch ctl`).

**Docs / ADRs.** Update `docs/integrations.md`, `README.md`, `docs/design.md`, and the
`AGENTS.md` crate list (birch-ctl is no longer a crate; control is a `birch` subcommand). ADRs
0010/0011 reference `birch-ctl` as the client — note the rename. Record the decision in a new
ADR (control client is a `birch` subcommand, not a separate binary; tmux model; one binary and
one formula).

**Release.** With one binary, cargo-dist emits a single `birch` app → one `birch.rb`. Remove
`birch-ctl` dist metadata and regenerate; delete the obsolete hand-rolled
`packaging/homebrew/render_formula.py`.

**Verification.** `cargo build` / `test` / `clippy --all-targets` / `fmt`; `birch --help`,
`birch ctl --help`, `birch <dir>`, and a live `birch ctl get-root` roundtrip against a running
instance; `dist plan` shows a single `birch` app/formula.
