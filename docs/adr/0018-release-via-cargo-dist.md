---
type: ADR
title: Release automation and the Homebrew tap via cargo-dist
status: Accepted
sprint: sprint-013
supersedes: 0017-prebuilt-binaries-and-homebrew-tap
---

# Context

[ADR 0017](0017-prebuilt-binaries-and-homebrew-tap.md) chose to hand-roll the release pipeline
(a custom GitHub Actions matrix plus a formula generator) on the grounds that cargo-dist's
upkeep was uncertain after Axo wound down. That premise was wrong: cargo-dist is actively
maintained (0.32.0, May 2026; issues worked in June 2026) and its fork was adopted by Astral.
Releasing a Rust CLI as prebuilt binaries with a Homebrew tap is the exact commodity use case
cargo-dist automates, so hand-rolling put the project needlessly off the beaten path.

# Decision

Use **cargo-dist** for release automation. `cargo dist init` generates the tag-triggered
`release.yml`, builds prebuilt binaries for the target matrix, publishes a GitHub Release, and
generates and publishes a single Homebrew formula (one binary, `birch`, with `birch ctl` as its
control subcommand — see [ADR 0019](0019-control-client-is-a-birch-subcommand.md)) to
`slukyano/homebrew-tap`. Configuration lives in `dist-workspace.toml` (`[dist]`) plus
`[package.metadata.dist]` on the distributed crate; the release workflow is generated, not
hand-edited (regenerate with `dist generate`).

Targets: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu` (extend via
config). The contrib adapters ship alongside the binaries via dist's `include`. The tap push
uses a PAT stored as a secret, per cargo-dist's documented Homebrew flow.

The hand-rolled `release.yml` and `packaging/homebrew/render_formula.py` are removed. CI
(`.github/workflows/ci.yml`) is unaffected.

# Consequences

- On the beaten path: standard tooling, generated config, community support, and no bespoke
  formula renderer to maintain.
- cargo-dist owns `release.yml` and adds `[workspace.metadata.dist]` config; changes go through
  regeneration, not hand edits — the standard turnkey trade.
- A cargo-dist version is pinned in the generated workflow; bumping it is a deliberate step.
- Adding targets or installers is a config change plus a regenerate.

# Alternatives considered

- **Hand-rolled (ADR 0017)** — superseded; needlessly bespoke for a commodity case.
- **GoReleaser** — mature, language-agnostic, first-class Homebrew support, but Go-centric
  config; for a pure-Rust project cargo-dist is the closer native fit.
