---
type: Task
title: Canonicalize symlinks before the reveal root-containment check
description: '`birch ctl reveal /tmp/foo` is rejected as "outside the root" when the root is /private/tmp (macOS /tmp → /private/tmp).'
status: Done
priority: high
---

Found in the Sprint 014 new-user DX review. The `reveal` (and likely `set-root`) containment check
compares the incoming path against the tree root **without canonicalizing symlinked prefixes**, so
on macOS (`/tmp` → `/private/tmp`) a very common case fails:

```
# root = /private/tmp
$ birch ctl reveal /tmp/acme/src/main.rs
birch ctl: path is outside the root        # exit 1
$ birch ctl reveal /private/tmp/acme/src/main.rs
# works, exit 0
```

This breaks the headline reverse-integration workflow the README advertises
(`birch ctl reveal src/main.rs` from an editor/adapter), since editors routinely pass `/tmp/...`
(and other symlinked) paths.

Fix: canonicalize the incoming path — or its existing ancestor prefix, since the leaf may not
exist yet — with the same normalization applied to the root before the containment test, so
symlinked prefixes match. The client absolutizes against its own cwd (`ctl_client.rs::absolutize`);
the containment check lives on the server side (the reveal handler in `crates/birch/src/ctl.rs` /
`birch-core`). Add a test covering a symlinked-prefix path (`/tmp` vs `/private/tmp`). Confirm
`set-root` handles the same case.

## Design

**Root cause (confirmed).** `main.rs:136` canonicalizes the root at launch; the reveal handler
(`app.rs:462-478`) only **lexically** normalizes the incoming path (`lexical_normalize`, by design —
its comment keeps an *in-tree* symlink from resolving outside the tree) and then tests
`abs.starts_with(self.root)`. A root-*prefix* symlink (`/tmp → /private/tmp`) therefore never
matches the canonical root. (`set-root` at `app.rs:583` already canonicalizes, so it is unaffected;
this is a reveal-only fix.)

**Fix (match as-given first; canonicalize only as a rescue).** Do not canonicalize eagerly — take
the path the caller gave and only resolve symlinks when it doesn't already match:

1. Lexically normalize the input to `abs` (as today — resolves `..`, no symlink resolution).
2. If `abs.starts_with(root)` → **reveal `abs`**. Covers relative inputs, already-canonical
   absolutes, and **in-tree symlink nodes referenced by their listed path** — no canonicalization,
   so their identity is preserved (this is why nothing regresses).
3. Otherwise (the `/tmp → /private/tmp` case): canonicalize `abs` — or its longest existing
   ancestor, re-appending the not-yet-existing remainder — and if the result is under `root`,
   reveal it; else `path is outside the root`.

So symlink resolution is a **fallback that only fires for a path the tree didn't already match**;
the common and in-tree cases never hit it. `set-root` already canonicalizes its argument
(`app.rs:583`), so it is unaffected — this is a reveal-only fix.

**Public surface.** None — a behavior fix to the existing `reveal` verb (a previously-rejected path
now resolves). No new flags, config keys, protocol fields, or public APIs.

**Tests:** an already-under-root absolute reveals via step 2 (no canonicalization); a symlinked
root prefix (`/tmp` vs `/private/tmp`) reveals via the step-3 fallback; a genuinely outside path
still errors; a not-yet-existing leaf under a symlinked root resolves via its ancestor.
