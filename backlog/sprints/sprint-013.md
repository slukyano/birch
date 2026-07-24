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

- [ ] 038-add-ci-workflow
- [ ] 042-set-up-homebrew-tap
- [ ] 043-automate-releases

# Open questions

Design inputs to resolve in the design phase: the release/packaging approach (build the formula
from source vs. from a release tarball with a pinned checksum), likely recorded as an ADR; the
CI target matrix and MSRV stance; and the maintainer touchpoint to create the `homebrew-tap`
repository.

# Session log

- Scoped and cut: three tasks — CI workflow, Homebrew tap/formula, and release automation.
  Branch `sprint/013` cut from `main`; README install docs (`044`) and the docs restructure
  (`049`) held for a later docs sprint.
