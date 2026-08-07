---
type: Task
title: Add in/out badges for branch changes
description: Badges in the git gutter marking files whose changes are committed on this branch but not on main, and the reverse.
status: Draft
priority: medium
---

Maintainer request: in/out badges on files, behaving like the git status badges but representing
whether a file's changes are **committed on the current branch** as against `main`.

Today's badges (`FileStatus`: conflicted, deleted, renamed, modified, added, untracked) come from
`git status --porcelain=v2` and describe the *working tree and index against `HEAD`* — what has not
been committed yet. This asks the opposite question: what *has* been committed here, and how does
that branch differ from its base. The two are orthogonal, and a file can carry both.

## What "in/out" means

The obvious reading is incoming and outgoing, which are two different queries:

- **out** — the file changed in commits on this branch that the base does not have
  (`git diff --name-status $(git merge-base main HEAD)..HEAD`);
- **in** — the file changed on the base since the branch diverged, so a merge or rebase will bring
  those changes in (`git diff --name-status HEAD...main`).

Whether both directions are wanted, or only *out*, is the first design question — the request
emphasises "committed in the current branch vs main", which is *out* alone, but the name implies
both. `in` is the more novel half: it warns that a file being edited has moved underneath the
branch.

## The hard parts

- **What "main" is.** The base is not always `main`: it may be `master`, a repository's configured
  default branch, the upstream tracking branch, or a stacked branch's parent. Falling back through
  those, and behaving sanely when none resolves (detached `HEAD`, a fresh repo with no commits, a
  branch with no merge-base), is most of the work.
- **Refresh.** `GitWorker` recomputes on `GitCmd::Refresh`, which the app sends on filesystem
  change. Branch state changes on events the tree never sees — `git commit`, `checkout`, `rebase`,
  `fetch` — which live inside `.git`, a directory birch treats as noise and does not watch. Either
  something watches `.git/HEAD` and the refs, or the badges go stale until an unrelated file
  changes.
- **Cost.** A `merge-base` plus a diff per refresh is more expensive than the single `status` call
  today, and it is wasted work in a repo where the branch *is* the base — the overwhelmingly common
  case, which should cost nothing.
- **Where the badges go.** `render::BADGE_WIDTH` is 2 columns and the git status badge already owns
  them. A second, orthogonal badge either needs another column, shares one by precedence, or is
  distinguished by colour alone. This collides directly with
  [`064`](064-configurable-badge-placement.md) (badge placement) and
  [`068`](../archive/068-add-scrollbar.md), which took the far-right column for the scrollbar.
- **Ancestor propagation.** Status badges roll up to collapsed directories (design doc, Files
  source). Branch-diff badges presumably should too, and the rollup has to distinguish the two
  kinds rather than merging them into one "something happened here" mark.

## Scope

Not on the permanent out-of-scope list, and it extends an existing whitelisted feature rather than
introducing a new one — the design doc's Files source already promises "git status badges + colors"
with ancestor propagation. Worth confirming with the maintainer that a second badge *axis* counts
as that feature growing rather than a new one.
