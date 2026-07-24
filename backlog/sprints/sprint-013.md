---
type: Sprint
title: Installable & CI-guarded
status: Done
branch: sprint/013
tasks:
- 038-add-ci-workflow
- 042-set-up-homebrew-tap
- 043-automate-releases
- 050-unify-control-client
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
- [x] 042-set-up-homebrew-tap
- [x] 043-automate-releases
- [x] 050-unify-control-client

# Open questions

Design inputs (resolved): release/packaging approach recorded as
[ADR 0017](../../docs/adr/0017-prebuilt-binaries-and-homebrew-tap.md) — prebuilt binaries;
CI on Linux + macOS, stable, no MSRV job; tap named `homebrew-tap`.

Stop-and-ask (implementation): `042`/`043` are blocked on two maintainer touchpoints — creating
the `slukyano/homebrew-tap` repository, and adding a `HOMEBREW_TAP_TOKEN` secret so the release
workflow can push the formula bump. `038` is implemented and locally green; the release pipeline
cannot be verified end-to-end until the touchpoints are resolved.

# Sprint summary

- **038-add-ci-workflow** (mid): `.github/workflows/ci.yml` — `fmt --check` on Linux, and
  `clippy --all-targets -- -D warnings` + `test` across ubuntu + macos, on push to `main` and PRs.
- **050-unify-control-client** (major; ⚠️ added mid-sprint): the separate `birch-ctl` binary/crate
  folded into a `birch ctl <verb>` subcommand ([ADR 0019](../../docs/adr/0019-control-client-is-a-birch-subcommand.md);
  tmux model). `main` pre-dispatches on `argv[1] == "ctl"`, leaving `birch [DIR]` untouched;
  adapters call `birch ctl`; docs updated. Removed so cargo-dist emits one formula.
- **042-set-up-homebrew-tap** (major; ⚠️ transformed): `slukyano/homebrew-tap` created; the formula
  is generated and pushed by **cargo-dist** ([ADR 0018](../../docs/adr/0018-release-via-cargo-dist.md)),
  not the hand-rolled formula first designed. One `birch.rb`.
- **043-automate-releases** (major; ⚠️ transformed): release automation is cargo-dist's generated
  `release.yml`, not the hand-rolled matrix. On a `v*` tag it builds three targets, publishes a
  GitHub Release, and pushes the formula to the tap.

**⚠️ Transformation:** the release approach pivoted from a hand-rolled pipeline to **cargo-dist**
(ADR 0018 supersedes 0017) after the "cargo-dist is unmaintained" premise proved false (it is
Astral-maintained). That pivot forced the `050` control-client unification (one binary → one
formula).

**Delivered & live-verified:** birch **v0.1.0** is published — public repo, GitHub Release (three
platforms + checksums + shell installer), and `Formula/birch.rb` in the tap. `brew install
slukyano/tap/birch` resolves, downloads, and checksum-verifies the artifact (the final extract on
the maintainer's machine was blocked only by an outdated Xcode CLT, unrelated to birch).

**Not done (deferred, named):** `044-document-installation` and `049-dedup-and-route-docs` (docs
sprint); `045`/`046`/`047`/`048` (polish).

**Breaking change:** `birch-ctl` is gone — control is now `birch ctl <verb>`, and `$BIRCH_CTL`
is replaced by `$BIRCH`. In-tree adapters updated; any external caller must switch to `birch ctl`.

**Architectural decisions:** ADR 0018 (release via cargo-dist; supersedes 0017) and ADR 0019
(control client is a `birch ctl` subcommand) — both Accepted.

**Bugs found & fixed:** hand-rolled "release not found" and checksum-filename issues (both moot
after the cargo-dist pivot); review nits (`birch ctl --version` parity; ADR 0018 config-path
wording); and the **empty-tap** failure (`couldn't find remote ref refs/heads/main`) — fixed by
seeding the tap with an initial commit (recorded in `042`).

**Remaining limitations & highlights (must-read):**
- **Adapters install to `share/birch/`, not `PATH`.** cargo-dist puts included non-binary files in
  pkgshare, so after `brew install` the adapters live at `$(brew --prefix)/share/birch/`, not
  callable by bare name. cmux (absolute path in `dock.json`) is unaffected; tmux/herdr bare-name
  bindings need the share path or a symlink. A candidate follow-up.
- First release requires a **seeded tap** (empty repos fail cargo-dist's checkout); **prereleases
  skip** the formula push unless `publish-prereleases = true`.

# Session log

- Scoped and cut: three tasks — CI workflow, Homebrew tap/formula, and release automation.
  Branch `sprint/013` cut from `main`; README install docs (`044`) and the docs restructure
  (`049`) held for a later docs sprint.
- Course correction (maintainer-directed): the hand-rolled release pipeline of the original
  design was replaced with **cargo-dist** (ADR 0018 supersedes 0017) — releasing a Rust CLI with
  a Homebrew tap is cargo-dist's commodity case, and cargo-dist is actively maintained (Astral).
  `038` (CI) landed as designed. Adopting cargo-dist surfaced that its per-package model would
  emit two formulae (`birch`, `birch-ctl`); rather than special-case it, added `050` to fold the
  control client into a `birch ctl` subcommand (tmux model) — one binary, one formula — then
  finish `042`/`043` on cargo-dist.
- Verified end-to-end and closed out: gates green (115 tests), independent review clean (two nits
  fixed), the cargo-dist pipeline live-verified via an RC and then the real **v0.1.0** release
  (build → Release → formula pushed to the tap → `brew install` resolves/verifies). Tasks and
  sprint flipped Done; ADRs 0018/0019 Accepted, 0017 Superseded.
