---
type: Task
title: birch-tmux and birch-herdr bake a raw $0 into --open-cmd
description: Both write $0 into birch's open command with no resolution, so a bare or relative invocation produces a callback that may not resolve.
status: Draft
priority: low
---

`contrib/birch-tmux:65` and `contrib/birch-herdr:57` build birch's `--open-cmd` from `$0` directly:

```sh
open_cmd="$(shquote "$0") open-in-main {}"
```

`birch-cmux` does not — `:42-47` resolves `$0` through `command -v` and absolutises it against
`$PWD`, because the callback has to work from birch's cwd, not the adapter's. The other two have the
same exposure and none of the protection.

Since [ADR 0027](../../docs/adr/0027-adapters-ship-as-binaries.md) all three arrive through the same
carrier, which hands the script its own argv[0] (`crates/birch/src/adapter.rs:28-32`). So the fix
belongs there: resolve argv[0] to an absolute path in Rust — PATH lookup for a bare name, `$PWD`-join
for a relative one — and all three scripts inherit it.

That keeps ADR 0027's decision intact (argv[0], not `current_exe()`, so a PATH install resolves to
the stable `…/bin/birch-cmux` rather than a `Cellar/0.2.0/…` path that `brew cleanup` deletes), and
it does **not** change the script text, so the scripts stay directly runnable and comparable to what
ships — which ADR 0027 promises.

Note [`083`](../archive/083-birch-cmux-does-three-things.md) removes `birch-cmux`'s own use of `$0` entirely, so
after that lands this is purely about the other two.

Costs to weigh: each script gains a line of comment noting the carrier hands it an absolute `$0`,
and a directly-run relative invocation (`sh ./contrib/birch-tmux open`) loses the fixup unless the
shell version is kept as well. That is one sentence of coupling between carrier and script — less
than `birch-cmux` carries today.

## Rejected while here

- **A shared shell library** sourced by all three. The genuinely identical code is about twelve lines
  per script; a library costs a resolution problem with no good answer for the binary form, for a
  copied-out script, and for an out-of-tree adapter — and destroys the property that each file is a
  complete runnable example, which `docs/design.md` says is their purpose.
- **A prelude prepended inside `adapter::run`.** ADR 0027 newly makes it possible, and it should
  still be refused: it makes the shipped binary and the checked-in script behave differently, which
  breaks ADR 0027's own "the pair can be compared at any time" and the test harness's script
  fallback.
- **Anything in birch that knows about hosts** — a `--follow-cmd`, a re-root poller, a "cmux mode".
  That is the plugin instinct `docs/design.md` fences off.

Still open, and a maintainer decision rather than a cleanup: **`birch ctl socket-path <key>`**, which
would let birch own the private state directory, the ownership check, the `stat -f`/`stat -c`
platform split, and the socket-length hashing — replacing nine lines and two forks in each of three
scripts, and fixing the `mkdir` race and the misleading `stat` error in one place instead of three.
Against it: `docs/design.md` says birch's promise is four things and "nothing else", and this is a
fifth, permanent one.
