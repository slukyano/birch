---
type: Task
title: Add the MIT LICENSE file
description: Add a root LICENSE (MIT) matching Cargo's license = "MIT"; without it GitHub reads the repo as all-rights-reserved.
status: Draft
priority: high
---

`Cargo.toml`'s `[workspace.package]` declares `license = "MIT"`, but there is no
`LICENSE` file at the repository root, so GitHub shows "no license" — which legally means
all rights reserved and blocks reuse. Add the standard MIT license text with the
copyright line for Stanislav Lukyanov. Trivial; no design.
