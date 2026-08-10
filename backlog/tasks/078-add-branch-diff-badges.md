---
type: Task
title: Add in/out badges for branch changes
description: Badges in the git gutter marking files whose changes are committed on this branch but not on main, and the reverse.
status: Designed
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

One extra column marking the files this branch would merge — the scope of the change, not its
shape.

```
branch badges off                    branch badges on
    src                     ●            src                   ⎇ ●
    │  app.rs               M            │  app.rs             ⎇ M
    │  input.rs                          │  input.rs           ⎇
    │  render.rs                         │  render.rs          ⎇
    │  main.rs              M            │  main.rs              M
    README.md               A            README.md              A
```

- **One symbol, one meaning.** `⎇` says "this file differs from the base" — it is in scope for the
  merge. It carries no letter: whether the change is an add or a modify is already in the
  working-tree column when it matters, and one keystroke away in a diff when it does not. A second
  vocabulary of letters would have to be learned and told apart from the first at a glance.
- **The local badge is outermost and never moves.** The mark is added *inward*, and the space
  between the two keeps the letter a user already reads in exactly its column.
- **Rollups as today.** A collapsed directory carries the mark if anything beneath it is in scope,
  so a folded tree still shows where the branch's work is.
- **A branch icon where one exists, a plain glyph where it does not.** With `--icons` the mark is
  the **codicon git-branch** (`U+EA68`), matching the icon family the rest of the tree draws;
  without icons it is **`∆`** (`U+2206`), which says "difference". `⎇` (`U+2387`) was the first
  choice and was withdrawn: measured against the `cmap` tables of Menlo, SF Mono, JetBrains Mono,
  Courier New and Andale Mono, **no common terminal font contains it** — it renders on macOS only
  because CoreText silently borrows it from a proportional system font, which is the substitution
  that sheared the indent guides in
  [`059-fix-guide-chevron-alignment`](../archive/059-fix-guide-chevron-alignment.md), and on a bare
  Linux box it is a tofu box. An arrow was never a candidate: `↑` universally means "committed but
  not pushed".
- **Cost: two columns**, taken from filenames — which is why it is a setting.

**One thing it cannot show.** A file *deleted* on this branch has no row to mark: it is absent from
disk, so the tree has nothing to draw. That is true of any design here, and it means the column
answers "what would I be merging, of what exists" rather than "what does this branch change".

**Only the base comparison.** Comparing against this branch's own remote (committed but unpushed)
was left out: it is nearly always a subset of the base comparison, so it repeats information for
another column. Comparing against a *local* `main` was rejected outright — a pull request is opened
against the remote, and local `main` is usually the staler of the two.

### What the base is

The remote's default branch: `origin/HEAD`, else `origin/main`, else `origin/master`. Not a local
branch — the comparison exists to answer a question about a pull request, and local `main` answers
a different one.

When no remote base resolves — no remote, a detached `HEAD`, a repository with no commits — **the
axis is simply absent**: no badges, no error, the gutter back to two columns.

**The base is only as fresh as the last fetch, and birch will not fetch.** A comparison can be hours
stale with nothing on screen to say so, so the root row names the base and its age —
`birch  ~/work/birch  (vs origin/main, fetched 2h ago)` — and the badges carry their own caveat.

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

### Glyph coverage, measured

The `cmap` tables of Menlo, SF Mono, JetBrains Mono, Courier New and Andale Mono were parsed
directly, and the candidates' advance widths read from `hmtx` against the font's own cell:

- **No branch-shaped glyph is in any of them** — `⎇` U+2387, `⋔` U+22D4, `⑂` U+2442 all absent.
- **The codicon git-branch (U+EA68) advances exactly one cell** (600/1000 em, the same as `M`) in
  both the Mono and standard JetBrains Nerd Font variants, as do the octicon, devicon and powerline
  branch glyphs. It is therefore safe wherever a Nerd Font is in use — which `--icons` already
  presumes.
- **`∆` U+2206 is in all five**, is Neutral width, and needs no substitution.

### Open

Whether the base should be overridable per invocation (`--base <ref>`) rather than only resolved.
