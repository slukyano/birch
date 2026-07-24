---
type: Task
title: Add the MIT LICENSE file
description: Add a root LICENSE (MIT) matching Cargo's license = "MIT"; without it GitHub reads the repo as all-rights-reserved.
status: Done
priority: high
---

`Cargo.toml`'s `[workspace.package]` declares `license = "MIT"`, but there is no
`LICENSE` file at the repository root, so GitHub shows "no license" — which legally means
all rights reserved and blocks reuse. Add the standard MIT license text with the
copyright line for Stanislav Lukyanov.

## Design

Add a root `LICENSE` file with the verbatim OSI MIT text and the copyright line:

```
Copyright (c) 2026 Stanislav Lukyanov
```

The year is a real legal convention (distinct from the fabricated authoring timestamps that
were stripped elsewhere), so it stays. `Cargo.toml` already declares `license = "MIT"`
(SPDX), so no manifest change is needed; GitHub detects the root `LICENSE` and surfaces it in
the repo header. No other files change.

