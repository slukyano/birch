---
type: ADR
title: Defer crates.io publishing; Homebrew and cargo-install-from-git are the channels
status: Proposed
sprint: sprint-014
---

# Context

birch is distributed via a Homebrew tap ([ADR 0018](0018-release-via-cargo-dist.md)) and
`cargo install --git`. crates.io is a third possible channel, but the crate name `birch` is taken
([ADR 0002](0002-keep-the-name-birch.md)) and the workspace crates are `publish = false`.
Publishing would mean either a different crate name (`birch-tree`, free per ADR 0002, with
`[[bin]] name = "birch"`) or not shipping to crates.io at all.

# Decision

**Defer crates.io.** Homebrew (live) and `cargo install --git https://github.com/slukyano/birch
birch` already cover both non-Rust and Rust users. Keep the crates `publish = false`.

Publishing adds real upkeep — SemVer discipline across the workspace, a publish step on every
release, and a public API surface for the library crates (`birch-core`'s sources/deltas are the
most volatile interfaces, and the ones the design doc deliberately keeps unstable) — with no
external consumer asking for it today.

# Consequences

- No `cargo install birch-tree`; Rust users use `cargo install --git` (or Homebrew).
- Cargo metadata (`repository`, `homepage`, `keywords`, `categories`) is filled in regardless, so a
  crate is publish-ready if this reverses.
- Reversing is a new ADR: publish the binary crate as `birch-tree` with `[[bin]] name = "birch"`,
  flip its `publish`, and add a publish step.

# Alternatives considered

- **Publish `birch-tree` now** — enables `cargo install birch-tree`, but adds release/SemVer upkeep
  for a channel `cargo install --git` already covers.
- **Rename to claim `birch` on crates.io** — the name is taken; not available.
