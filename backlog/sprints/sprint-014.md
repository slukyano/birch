---
type: Sprint
title: Docs & publication polish
status: Done
branch: sprint/014
tasks:
- 044-document-installation
- 045-add-repo-demo
- 046-add-cargo-metadata
- 047-decide-crates-io-publish
- 048-add-changelog
- 049-dedup-and-route-docs
---

# Scope rationale

Close the publication arc now that birch is installable: a coherent documentation set, a README
that sells the tool, and the remaining publication polish.

- **Documentation architecture** (`049-dedup-and-route-docs`, design-heavy) — one home per topic
  across `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `backlog/workflow.md`, and `docs/`, with
  progressive-disclosure pointers; add a `docs/` index (and log); date the change logs; restructure
  `CONTRIBUTING.md`.
- **README content** — the Install section (`044-document-installation`: `brew install
  slukyano/tap/birch`, `cargo install --git`, the adapter-in-`share/` caveat) and a demo
  (`045-add-repo-demo`: asciinema/GIF).
- **Publication polish** — finish the Cargo metadata (`046`, only `keywords`/`categories` remain),
  decide the crates.io story (`047`, likely an ADR), and add a changelog (`048`).

Ordering: `049` lands first — it sets README's/CONTRIBUTING's/AGENTS's structure and roles; then
`044`/`045` fill the restructured README, and `046`/`047`/`048` are independent polish.

The adapter-on-PATH gap from Sprint 013 is captured as `051-install-adapters-on-path` and is **not**
in this sprint; `044` documents the current `share/birch/` location.

# Checklist

- [x] 044-document-installation
- [x] 045-add-repo-demo
- [x] 046-add-cargo-metadata
- [x] 047-decide-crates-io-publish
- [x] 048-add-changelog
- [x] 049-dedup-and-route-docs

# Open questions

Design inputs for `049` are captured in its body (CONTRIBUTING shared-core split; how much of a
bundle `docs/` becomes; dating format; repository-map depth; the exact cross-link wiring). `047` is
a decision (publish the binary crate as `birch-tree` per ADR 0002, or stay Homebrew-only) — likely
an ADR.

# Session log

- Scoped and cut: six tasks — documentation architecture (`049`), README install docs (`044`) and
  demo (`045`), and publication polish (`046`/`047`/`048`). Branch `sprint/014` cut from `main`.
  The adapter-on-PATH gap seeded as `051` and held out of scope.
- Done. `049` reorganized `docs/` and `backlog/` into OKF bundles and routed each topic to a single
  home (README / AGENTS / CONTRIBUTING / `workflow.md` / `docs/`), with a `docs/` index and dated
  logs. `044` added the Install and Quick Start sections; `045` a vhs demo GIF (opens on the tree,
  Nerd Font icons, git badges, fuzzy search); `046` filled `keywords`/`categories`; `047` deferred
  crates.io ([ADR 0020](../../docs/adr/0020-defer-crates-io.md) — Homebrew and `cargo install
  --git` are the channels); `048` added `CHANGELOG.md`. Alongside the scoped tasks: a README
  overhaul (`# birch` H1 with an outlined birch logo, CI/release/license badges, a Features list, a
  Quick-Start-first order, the "modern interactive file tree for the terminal" tagline, and a
  dedicated advanced `birch ctl` section). Three follow-ups were seeded from the DX review — `052`
  (reveal symlink canonicalization), `053` (state-persistence toggle), and `054` (tree visual
  polish); `025` was rescoped to style presets only. Gates green (`fmt`/`clippy`/`test`) and an
  independent diff review clean; branch squashed and merged to `main`.
