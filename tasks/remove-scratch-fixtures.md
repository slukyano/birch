---
type: Task
title: Remove tracked scratch fixtures and tighten .gitignore
description: Drop bar.md / bar2.md / foo/ from the tree and ignore scratch, editor, and tooling dirs.
status: Draft
priority: high
---

`bar.md`, `bar2.md`, and `foo/bar/foobar/d.md` are tree test scratch files that were
committed and are now public. Remove them and extend `.gitignore` to cover local scratch,
editor, and tooling artifacts (`.claude/`, `.cmux/`, `.readb`, and a scratch pattern) so
they cannot return. Design decision: whether to also purge them from the `prepare for
publication` root commit (amend + force-push) or just remove them going forward.

## Design

Remove the tracked scratch files with `git rm`: `bar.md`, `bar2.md`, and the entire `foo/`
tree (`foo/bar/foobar/d.md` and its now-empty parents). A plain commit on the branch — **no
history rewrite**. The files leave the working tree and every future revision; they remain
reachable only in the `prepare for publication` root commit, which is an accepted trade to
avoid a force-push on the freshly published repo.

Extend `.gitignore` with the local tooling and agent artifacts that must never be committed:

```
# Tooling / agent artifacts
.claude/
.cmux/
.readb
```

`.DS_Store` and `/target` are already ignored. No broad scratch glob (e.g. `*.md` under a
scratch dir) — it would risk masking real content; the stray fixtures were ad-hoc root files,
not a pattern worth generalizing.

No source, manifest, or test changes.

