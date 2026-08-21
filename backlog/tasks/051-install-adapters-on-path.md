---
type: Task
title: Install the contrib adapters on PATH
description: brew install puts the adapters in share/birch/, not PATH; make birch-tmux / birch-herdr / birch-cmux callable by bare name.
status: Designed
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

Sprint-019 note: the task's premise that "`birch-cmux` is unaffected" no longer holds. Its
`dock.json` absolute path is exactly the wart the maintainer's *"do a cmux integration **fully**"*
is about — it currently points into a git checkout and at a **debug** binary.

## Design

**cargo-dist offers no lever.** The generated formula ends with

```ruby
leftover_contents = Dir["*"] - doc_files
pkgshare.install(*leftover_contents) unless leftover_contents.empty?
```

and everything that is not a declared *binary* lands there. The config reference has no formula
template, `post_install`, or caveats hook — the same gap as
[axodotdev/cargo-dist#2457](https://github.com/axodotdev/cargo-dist/issues/2457). `bin-aliases`
only aliases existing binaries; `dist.binaries` only narrows the list per platform. So the adapters
reach `bin` only if cargo-dist *believes they are binaries*.

That leaves four real shapes. **This is a maintainer decision** — it changes what the `birch`
package ships, or how birch is released.

### A — the adapters become binaries of the `birch` package *(recommended)*

Add `src/bin/birch-cmux.rs` and siblings: each `include_str!`s the contrib script and runs it,

```rust
// $0 must be the wrapper's own path — the script bakes it into --open-cmd.
Command::new("/bin/sh").arg("-c").arg(SCRIPT).arg(current_exe).args(args)
```

so `sh` sees the script as `-c`, `$0` as the installed path, and `$@` as the verbs.

- **Every channel gains them at once** — brew, the shell installer, and `cargo install`, which
  ships no adapters at all today.
- **The scripts stay the source of truth**, still readable and copyable at `share/birch/` via the
  existing `include`, so the contrib-as-example story in `docs/design.md` is intact.
- **No upstream dependency** and nothing to redo when cargo-dist changes.
- Costs: the `birch` package grows from one binary to four (~1 MB of archive), and `contrib/`
  becomes a build input of `crates/birch`. Worth an ADR.

### B — a generic `dist:` workspace member

A `birch-adapters` package with `binaries = [...]` and a copying `build-command`. Keeps the
scripts as scripts, but produces a **second formula**: `brew install birch` would no longer be
enough, which trades one setup wart for another.

### C — hand-maintain `Formula/birch.rb`

Drop `installers = ["homebrew"]` and own the formula. Total control — `bin.install` the adapters,
and the `head` stanza the dev channel wanted. Costs the release automation: three targets' urls and
sha256s updated by hand every release. A real regression in hygiene.

### D — accept `share/birch/` and document it

The outcome this task explicitly permits. `$(brew --prefix)/share/birch/birch-cmux` is stable and
installed, so `dock.json` stops pointing into a checkout even though it stays a path. Cheapest, and
the only option that leaves `birch-tmux toggle` in the docs still wrong.

### Regardless of the choice: `birch-dev`

`Formula/birch-dev.rb` is **hand-written** in the tap, not generated, so it can `bin.install` the
adapters today under any of the above. That is what unblocks the maintainer's own `dock.json`,
which is the concrete thing in the way right now.
