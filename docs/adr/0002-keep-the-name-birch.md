---
type: ADR
title: Keep the name "birch"; Homebrew is the distribution channel of record
status: Accepted
sprint: sprint-001
---

# Context

The design doc flags name availability as an open question to resolve before the name
hardens into docs, the socket path scheme (`$XDG_RUNTIME_DIR/birch/`), and adapter names.
Availability check:

- **crates.io `birch`: taken** — a secret-rotation tool (first published 2025-11, v0.1.1,
  ~68 downloads).
- **Homebrew: free** — no formula or cask named `birch`.
- **Free on crates.io**: `birch-ctl`, `birch-core`, `birch-tui`, `birch-tree`, `birchtree`.

The workspace crates are `publish = false`, and per the design doc, publishing a library is
deliberately deferred.

# Decision

Keep the product, binary, and repository name **birch**. For a terminal tool the channel
that matters is Homebrew (formula installs a `birch` binary), and it is free. If cargo-install
distribution is ever wanted, publish the binary crate as **`birch-tree`** (free) with
`[[bin]] name = "birch"` — a crate/binary name mismatch is a minor, common wart. `birch-ctl`
is free everywhere and stays as-is.

# Consequences

- The name can harden now: socket paths, adapter names (`birch-herdr`), and docs are safe.
- crates.io `birch` belongs to an unrelated active project; birch never publishes under
  that name. Nothing reserves `birch-tree` — accepted risk, revisited only if cargo-install
  distribution becomes concrete.
