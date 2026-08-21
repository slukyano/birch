---
type: ADR
title: The host adapters ship as binaries that carry their script
status: Proposed
sprint: sprint-019
---

# Context

The three contrib adapters (`birch-cmux`, `birch-tmux`, `birch-herdr`) are shell scripts, and
`docs/design.md` treats them that way deliberately: a screenful of code a host owner can read, copy,
and adapt. Nothing about that should change.

What did not work was **getting them onto `PATH`**. cargo-dist installs declared *binaries* to `bin`
and sweeps everything else into `pkgshare`:

```ruby
leftover_contents = Dir["*"] - doc_files
pkgshare.install(*leftover_contents) unless leftover_contents.empty?
```

so after `brew install` the adapters sit in `share/birch/`. The docs' `birch-tmux toggle` binding
does not work, and a cmux `dock.json` can only name an absolute path into a particular install — in
practice a path into a git checkout, which is what the maintainer's own config had. `cargo install`
ships them nowhere at all.

There is no lever in cargo-dist: it generates the formula and rewrites it every release, and its
config reference has no formula template, `post_install`, or caveats hook — the same gap as
[axodotdev/cargo-dist#2457](https://github.com/axodotdev/cargo-dist/issues/2457). `bin-aliases` only
aliases binaries that exist; `dist.binaries` only narrows the per-platform list. The adapters reach
`bin` only if cargo-dist believes they *are* binaries.

The alternatives were weighed in `051`: a second generic package (and so a second formula, breaking
`brew install birch` as the whole story); hand-maintaining the formula (losing release automation —
three targets' urls and checksums by hand every release); or accepting `share/birch/` and
documenting the path (leaving the bare-name bindings wrong).

# Decision

- **Each adapter also ships as a binary of the `birch` package.** `crates/birch/src/bin/birch-<host>.rs`
  is three lines: it `include_str!`s `contrib/birch-<host>` and hands it to a shared helper.
- **The helper `exec`s `/bin/sh -c <script> <argv0> "$@"`.** `exec` rather than spawn-and-wait,
  because the adapters rely on being the process the host signals — a dock control dies with its
  pty's SIGHUP — and on their exit status reaching the caller.
- **`$0` is our own argv[0]**, with `current_exe()` only as a fallback. Every adapter bakes `$0`
  into birch's `--open-cmd` and re-execs itself through it, so it has to invoke the adapter again.
  argv[0] is what the scripts already expected — they resolve a bare name through `command -v`,
  landing on the stable `…/bin/birch-cmux`. `current_exe()` resolves symlinks and would pin the
  callback to a versioned path (`…/Cellar/birch/0.2.0/bin/…`), which `brew cleanup` deletes out
  from under a dock birch that is still running.
- **`dist-workspace.toml` drops its `include` of the three scripts.** `include` places files in
  the archive root under their basename — exactly where the binaries go — so shipping both would
  have two files claiming `<archive>/birch-cmux`. The binary is the one that has to win, and a
  second copy is redundant now that it carries the script. The readable original is `contrib/` in
  the repository, which the docs link to.

# Consequences

- **Every channel gains the adapters at once** — Homebrew, the shell installer, and `cargo install`,
  which shipped none. `dock.json` reads `birch-cmux dock-run`, and `birch-tmux toggle` works as
  documented.
- **`contrib/` becomes a build input of `crates/birch`.** Editing an adapter now requires a rebuild
  for the binary to change, and a stale `target/` can disagree with the script. The scripts remain
  directly runnable, so the pair can be compared at any time.
- **`include_str!` reaches outside the package directory**, which `cargo package` would not carry.
  `birch` is `publish = false`, and cargo-dist builds from the workspace checkout, so nothing in the
  release path depends on packaging the crate.
- **Three more binaries in the archive** (~1 MB), and the `birch` package now declares four —
  which is what carries them into the formula: cargo-dist's `homebrew.rb.j2` emits `bin.install`
  over every executable in the archive fragment, and it reads that list from cargo metadata.
- **No upstream dependency.** Nothing here needs cargo-dist to grow a feature, and nothing has to be
  redone if it does.
- birch's adapter promise is unchanged — the contract is still `--socket`, `--open-cmd`,
  `--open-detached`, and the socket protocol. Only the delivery changes.
