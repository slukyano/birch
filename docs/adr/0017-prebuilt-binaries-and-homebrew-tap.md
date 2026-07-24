---
type: ADR
title: Distribute prebuilt binaries via a Homebrew tap, built on tag
status: Accepted
sprint: sprint-013
---

# Context

[ADR 0002](0002-keep-the-name-birch.md) fixed Homebrew as the distribution channel of record but
left *how* the formula obtains birch open. Two options: a build-from-source formula
(`depends_on "rust" => :build`, `cargo install`) or a formula that installs prebuilt binaries
from a GitHub Release. Build-from-source is the least effort but makes every `brew install` a
multi-minute compile that needs a Rust toolchain — a poor first impression for a tool that wants
adoption.

The popular turnkey tool that generates both the release binaries and the tap formula
(cargo-dist / "dist") lost its maintaining company (Axo), so its upkeep is uncertain and it is
not a safe foundation.

# Decision

Distribute **prebuilt binaries**. On a version tag, a GitHub Actions workflow builds per-platform
archives, publishes them to a GitHub Release with SHA-256 checksums, and updates the Homebrew
formula in `slukyano/homebrew-tap` to point at the new archives. `brew install
slukyano/tap/birch` then installs a prebuilt `birch` (plus `birch-ctl` and the contrib adapters)
with no Rust toolchain and no compile.

Build on maintained, tool-agnostic building blocks — a GitHub Actions build matrix (e.g.
`taiki-e/upload-rust-binary-action`) plus a formula-bump step — rather than cargo-dist.

Initial target matrix: `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`.

# Consequences

- Fast, toolchain-free installs — the adoption-friendly path.
- Release infrastructure to own: a tag-triggered workflow, a per-platform build matrix,
  checksums, and an automated formula bump in the tap. The formula and release automation are
  coupled by design.
- A source build stays available (`cargo install --git`, `cargo build`); the prebuilt path is an
  addition, not a replacement.
- Adding platforms later means extending the matrix and the formula's per-platform blocks.
  `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-gnu` are candidates once demand appears.

# Alternatives considered

- **Build-from-source formula** — simplest, no release infra, but slow installs that need Rust;
  rejected for the adoption goal.
- **cargo-dist / dist** — turnkey for exactly this, but its post-Axo maintenance is uncertain; not
  chosen as a foundation, though its generated shape informs this design.
