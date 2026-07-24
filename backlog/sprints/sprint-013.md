---
type: Sprint
title: Installable & CI-guarded
status: Implementing
branch: sprint/013
tasks:
- 038-add-ci-workflow
- 042-set-up-homebrew-tap
- 043-automate-releases
---

# Scope rationale

Infrastructure to make the public repo CI-guarded and installable across machines. Three
tasks:

- a GitHub Actions **CI** workflow enforcing the validation gates (`fmt --check`,
  `clippy --all-targets`, `test`) on push and pull request;
- a personal **Homebrew tap** and formula (the distribution channel of record,
  [ADR 0002](../../docs/adr/0002-keep-the-name-birch.md)) installing the binary and the
  contrib adapters; and
- tag-driven **release automation** building cross-platform binaries into a GitHub Release.

Homebrew and releases are designed together — the formula's build-from-source vs.
release-tarball choice couples them. CI is independent and lands first to guard the rest. The
README install docs (`044-document-installation`) and the documentation restructure
(`049-dedup-and-route-docs`) are held for a later docs sprint.

# Checklist

- [x] 038-add-ci-workflow
- [ ] 042-set-up-homebrew-tap
- [ ] 043-automate-releases

# Open questions

Design inputs (resolved): release/packaging approach recorded as
[ADR 0017](../../docs/adr/0017-prebuilt-binaries-and-homebrew-tap.md) — prebuilt binaries;
CI on Linux + macOS, stable, no MSRV job; tap named `homebrew-tap`.

Stop-and-ask (implementation): `042`/`043` are blocked on two maintainer touchpoints — creating
the `slukyano/homebrew-tap` repository, and adding a `HOMEBREW_TAP_TOKEN` secret so the release
workflow can push the formula bump. `038` is implemented and locally green; the release pipeline
cannot be verified end-to-end until the touchpoints are resolved.

# Session log

- Scoped and cut: three tasks — CI workflow, Homebrew tap/formula, and release automation.
  Branch `sprint/013` cut from `main`; README install docs (`044`) and the docs restructure
  (`049`) held for a later docs sprint.
