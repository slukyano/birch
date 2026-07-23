---
type: Task
title: Automate tagged releases
description: Tag-driven GitHub Actions building macOS/Linux binaries and attaching them to a GitHub Release.
status: Draft
priority: medium
---

On a version tag, a GitHub Actions workflow builds release binaries (macOS arm64/x86_64,
Linux x86_64) and attaches them to a GitHub Release. The artifacts feed the Homebrew
formula (`set-up-homebrew-tap`) and give a no-toolchain install path. Design: the target
matrix, static/musl for Linux, whether the `contrib/` adapters ship in the tarball, and
how the formula pins checksums.
