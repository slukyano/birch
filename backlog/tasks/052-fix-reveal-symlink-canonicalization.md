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
