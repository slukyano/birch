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

## Design

### What the user sees

The VS Code model, extended by one question. A file that is modified in the working tree still shows
its `M`; a file whose change is **already committed on this branch but absent from the base** shows
the same letter, marked so the two cannot be confused — this second mark answers "what would I be
merging?"

The mark is a small glyph beside the letter rather than a different alphabet: `↑M` reads as "an `M`
that is going out", where a bare `M` is "an `M` sitting in my working tree". One vocabulary of
letters, one modifier that says which side of the comparison it belongs to. A separate in/out symbol
set was rejected — two alphabets in adjacent columns is a legend to memorise.

A collapsed directory rolls its branch state up the same way it rolls up working-tree state, so a
folded tree still shows where the branch's changes are.

**This costs gutter width.** Today the gutter is two columns and paints one. The branch axis needs
two cells of its own — the modifier and the letter — so with branch badges on, the right-hand
furniture becomes `↑M M` and the gutter grows from 2 columns to 4. That is real estate taken from
names in a narrow side pane, which is why the axis is a setting and why it is absent whenever no
base resolves.

**Outgoing only, in this task.** `git diff --name-status <merge-base>..HEAD` — what this branch has
that the base does not, which is exactly "what would I be merging". The incoming direction (what the
base gained since the branch diverged) answers a different question, needs its own modifier, and
would take a third pair of columns; it is deferred rather than crammed in.

### What "main" is

Resolved in order, first hit wins: the branch's upstream tracking ref; then the remote's default
branch (`origin/HEAD`); then a local `main` or `master`. When none resolves — a detached `HEAD`, a
repository with no commits, a branch with no merge-base — **the axis is simply absent**, no badges
and no error, exactly as `--no-git` renders. The resolved base is reported in the root row's
annotation so the comparison is never a mystery.

### Refresh

Branch state changes on `commit`, `checkout`, `rebase`, `fetch` — none of which touch the working
tree the watcher observes, and all of which live inside `.git`, a directory birch treats as noise.
The git worker gains a watch on `.git/HEAD` and the ref roots, so the badges refresh when the branch
moves. This is the one place the task touches the watcher, and it is deliberately narrow: two paths,
not a recursive watch of `.git`.

### Cost

A `merge-base` plus a `diff --name-status` per refresh, on top of today's `status`. When `HEAD` is
already the base — the common case — the diff is empty and the work is one `merge-base` call.
Results are cached against the resolved base commit and the branch tip, so a refresh triggered by an
unrelated file change reuses them.

### Public-surface delta

- **CLI**: `--no-branch-badges` (with the hidden bidirectional `--branch-badges`).
- **Config**: `branch-badges = <bool>`, default on where a base resolves.
- **Socket**: `SettingKey::BranchBadges`, an on/off/toggle value — additive.

### Open

Whether the incoming direction lands in a follow-up task, and whether the base should be
overridable per invocation (`--base <ref>`) rather than only resolved.
