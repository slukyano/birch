---
type: Task
title: Canonicalize symlinks before the reveal root-containment check
description: `birch ctl reveal /tmp/foo` is rejected as "outside the root" when the root is /private/tmp (macOS /tmp → /private/tmp).
status: Draft
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

**Fix.** For the **containment test only**, canonicalize the incoming path (falling back to
canonicalizing its longest existing ancestor and re-appending the remainder when the leaf doesn't
exist yet); compare that to the canonical root. On success, reveal the **lexically-normalized**
path as today (reveal semantics unchanged). Extract a small helper (`canonical_within(root, path)`)
so the same normalization is used both places.

**One design decision (flagged).** Canonicalizing the incoming path for the test also resolves
symlinks *inside* the tree, so a reveal that escapes the root *via an in-tree symlink* (`root/link →
/etc`, `reveal root/link/x`) would change from **accepted → rejected**. Two options:
- **(A, recommended)** Accept that — never treat a physically-outside path as in-tree; simplest,
  arguably more correct, one helper.
- **(B)** Resolve only the prefix at/above the root (canonicalize the root's ancestor chain, keep
  in-tree components lexical) to preserve today's in-tree-symlink acceptance — more code, narrower.

Recommend **(A)**.

**Public surface.** None — a behavior fix to the existing `reveal` verb (a previously-rejected path
now resolves). No new flags, config keys, protocol fields, or public APIs.

**Tests:** `/tmp` vs `/private/tmp` prefix resolves and reveals; a genuinely outside path still
errors; a not-yet-existing leaf under a real (symlinked) dir resolves via its ancestor.
