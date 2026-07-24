---
type: Task
title: Set up the Homebrew tap and formula
description: Personal slukyano/homebrew-tap with a birch formula installing the binary and the contrib adapters.
status: Designed
priority: high
---

Homebrew is the distribution channel of record
([ADR 0002](../../docs/adr/0002-keep-the-name-birch.md)). Create a personal tap
(`slukyano/homebrew-tap`) with a `birch` formula so `brew install slukyano/tap/birch`
works on any Mac. The formula installs the `birch` and `birch-ctl` binaries and the
`contrib/` adapters (`birch-cmux`, `birch-tmux`, `birch-herdr`) — the piece `cargo
install` cannot deliver. Design: build-from-source vs. install from a release tarball
(couples to `automate-releases`), and how the adapters land on PATH.

## Design

Per [ADR 0017](../../docs/adr/0017-prebuilt-binaries-and-homebrew-tap.md): a personal tap with a
prebuilt-binary formula, auto-updated by the release workflow (`043-automate-releases`).

- **Tap repo:** create `slukyano/homebrew-tap` (a maintainer touchpoint — `gh repo create`, or the
  maintainer creates it). Homebrew resolves `brew install slukyano/tap/birch` to
  `Formula/birch.rb` there.
- **Formula (`Formula/birch.rb`):** prebuilt strategy — `on_macos`/`on_linux` with
  `on_arm`/`on_intel` blocks giving the release archive `url` and `sha256` per platform. `def
  install` puts `birch` and `birch-ctl` in `bin` and the contrib adapters (`birch-cmux`,
  `birch-tmux`, `birch-herdr`) on `PATH` (`bin`, or `libexec` + wrappers). `test do` runs a smoke
  check (`birch --version`).
- **Updates:** the release workflow (043) rewrites the formula's URLs and checksums on each tag —
  no hand-editing; the initial formula is seeded pointing at the first release.
- **Adapters:** installed by the formula on every platform (they are shell scripts shipped in the
  release archive).

Depends on `043` for the artifacts and the bump automation; the two are implemented together.
Verification: `brew install slukyano/tap/birch` on a clean macOS installs a working `birch` plus
adapters; `brew test birch` passes.

