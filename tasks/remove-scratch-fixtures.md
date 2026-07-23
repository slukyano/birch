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
