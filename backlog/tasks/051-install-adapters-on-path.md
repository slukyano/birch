---
type: Task
title: Install the contrib adapters on PATH
description: brew install puts the adapters in share/birch/, not PATH; make birch-tmux / birch-herdr / birch-cmux callable by bare name.
status: Draft
priority: medium
---

Surfaced by Sprint 013's `v0.1.0` release: cargo-dist installs the `birch` binary to `bin` but
puts the `include`d contrib adapters (`birch-cmux`, `birch-tmux`, `birch-herdr`) in `share/birch/`
(its default for non-binary bundled files). So after `brew install slukyano/tap/birch` the adapters
are at `$(brew --prefix)/share/birch/`, not callable by bare name. `birch-cmux` is unaffected (its
`dock.json` uses an absolute path), but the `birch-tmux toggle` / `birch-herdr open` bare-name
bindings in the docs don't work off a brew install.

Goal: make the adapters available on `PATH` from a brew install (or decide they shouldn't be, and
document the `share/birch/` path instead).

Design challenge: cargo-dist **generates** the formula and regenerates it on every release, so a
hand-edited `def install` won't survive. Options to weigh: a cargo-dist-supported mechanism for
installing extra files to `bin`; a `post_install`/symlink approach expressed through config; a
small wrapper the formula can install; or accepting `share/birch/` and updating `integrations.md`
to reference that path (and `$BIRCH`-style resolution). Verify against a real `brew install`.
