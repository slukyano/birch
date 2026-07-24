---
type: Task
title: Automate tagged releases
description: Tag-driven GitHub Actions building macOS/Linux binaries and attaching them to a GitHub Release.
status: Done
priority: medium
---

On a version tag, a GitHub Actions workflow builds release binaries (macOS arm64/x86_64,
Linux x86_64) and attaches them to a GitHub Release. The artifacts feed the Homebrew
formula (`set-up-homebrew-tap`) and give a no-toolchain install path. Design: the target
matrix, static/musl for Linux, whether the `contrib/` adapters ship in the tarball, and
how the formula pins checksums.

## Design

Per [ADR 0017](../../docs/adr/0017-prebuilt-binaries-and-homebrew-tap.md): a tag-triggered release
workflow producing prebuilt archives and updating the tap.

- **Trigger:** push of a `v*` tag.
- **Build matrix:** `aarch64-apple-darwin` + `x86_64-apple-darwin` (macOS runner),
  `x86_64-unknown-linux-gnu` (Linux runner); release profile.
- **Package:** per target, a `tar.gz` of `birch`, `birch-ctl`, the three contrib adapters,
  `LICENSE`, and `README.md`; compute SHA-256.
- **Publish:** create the GitHub Release for the tag and attach the archives + checksums (a
  maintained action such as `taiki-e/upload-rust-binary-action` or `softprops/action-gh-release`).
- **Formula bump:** a final step rewrites `Formula/birch.rb` in `slukyano/homebrew-tap` with the
  new version, URLs, and checksums and pushes it (scoped token / deploy key — a maintainer
  touchpoint for the secret).
- **Versioning:** the tag drives the version, kept in step with the workspace `version`; cutting a
  release is `git tag vX.Y.Z && git push --tags`.

Couples to `042` (its formula is the bump target). `x86_64-unknown-linux-musl` and
`aarch64-unknown-linux-gnu` are deferred until demand appears (noted, not silently dropped).
Verification: a test tag yields a Release with three archives + checksums and a formula commit to
the tap; installing from the tap gives a working binary.

## Implementation note — cargo-dist ([ADR 0018](../../docs/adr/0018-release-via-cargo-dist.md))

The tag-driven release is cargo-dist's **generated `.github/workflows/release.yml`** (config in
`dist-workspace.toml`, `cargo-dist-version = "0.32.0"`), not the hand-rolled matrix above. On a
`v*` tag it builds the three targets (`aarch64`/`x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`),
packages each `.tar.xz` with the contrib adapters + LICENSE + README, publishes a GitHub Release,
and bumps the tap formula. The hand-rolled `packaging/homebrew/render_formula.py` was removed.

