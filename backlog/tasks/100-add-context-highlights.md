---
type: Task
title: Add context highlights
description: Generalise the branch column into a highlighted set chosen at runtime — a branch diff against any ref, recent changes, changes since a commit, or an explicit list.
status: Draft
priority: medium
tags:
- git
- socket
---

Request: generalise the branch-diff highlighting of
[`078-add-branch-diff-badges`](../archive/078-add-branch-diff-badges.md) so it can highlight
different sets of files, all of it controlled dynamically — settable at runtime by anything that
drives birch, an agent over the control socket in particular. Modes out of the box:

1. **Difference with another branch**, where both the branch and the remote can be given, so the
   base is not fixed to `origin/HEAD` → `origin/main` → `origin/master` but can be any ref. This is
   the `--base <ref>` question `078` left open.
2. **Files changed in the last N minutes.**
3. **Files changed since a given commit.**
4. **A custom highlight**: an explicit set of files handed over by the caller.

The branch diff stops being the feature and becomes one producer of a highlighted set among four.

## What exists

`078` built one column, inward of the working-tree badge, carrying one mark — the codicon
git-branch with `--icons`, `∆` without (`crates/birch-tui/src/render.rs:22-34`). The pieces:

- **The set.** `GitState::branch_changed` (`crates/birch-core/src/git.rs:52`) holds the changed
  files plus every ancestor below the repo root, so a collapsed directory carries the mark.
  `branch_changes` (`git.rs:329`) computes it with `git diff --name-only -z <base>...HEAD`.
- **The base.** `resolve_base` (`git.rs:278`) tries `origin/HEAD`, `origin/main`, `origin/master`,
  never a local branch, and always asks git by the fully qualified `refs/remotes/…` name — the
  shorthand would resolve a tag or a local branch of the same name.
- **The cache.** `BranchCache` (`git.rs:363`) reuses the set while `HEAD` and the base commit stay
  put, so a refresh triggered by an unrelated file change costs one `rev-parse` pair.
- **Refresh.** `reconcile_watches` (`crates/birch/src/app.rs:2824`) watches `.git`, `.git/refs`,
  `.git/refs/heads`, `.git/refs/remotes` and `.git/refs/remotes/origin` — non-recursively, and
  `origin` by name.
- **The explanation.** The root row's annotation names the base and its age —
  `(vs origin/main, fetched 2h ago)` (`crates/birch-tui/src/flat_view.rs:492`).
- **The surface.** `--no-branch-badges` / `--branch-badges`, config `branch-badges`, and
  `SettingKey::BranchBadges` under `set` (`crates/birch-core/src/protocol.rs:56`). All three are
  public and stay: the protocol is additive-only (ADR 0011), and the config file is a user's.

## Scope

**The panel's half.** VS Code exposes a file-decoration API through which an extension badges and
colours an arbitrary set of files in the explorer — the closest analogue of a set chosen by whoever
drives the panel. IntelliJ IDEA's project view offers scopes (`Changed Files` among them) and File
Colors keyed on scopes. **Verify both claims before designing**; the premise this task is filed on
is that the explorer decorating a caller-chosen set is panel behaviour, not an extension of it.

**The pane's half.** It adds no width if it takes over the column `078` already pays for, no modal
state, and no letter hotkey: the set is chosen over the socket, by a flag, or by config.

**The socket's fence.** A highlight changes what is drawn, never the filesystem, so it is not a
mutation verb. It is, however, a new verb in a set `docs/design.md` keeps small, and ADR 0035 is the
precedent for adding one when `set` is the wrong home.

**Not a source.** Boundary 1 (`docs/design.md`, Architecture) is about what the tree *contains*. A
highlight changes nothing about which rows exist; it is a side-table read at flatten time, as
`GitState` is (ADR 0005), and stays a paint-time transform keyed on real paths (boundary 2).
[`032-add-git-changes-source`](032-add-git-changes-source.md) — a tree of only changed files — is
the source-shaped neighbour, and could later reuse the same producers.

## The hard parts, per mode

- **Branch against any ref.** The ref has to resolve unambiguously (`upstream/develop` could be a
  remote branch or a local branch of that name), the watch list has to follow it — today a base on
  `upstream` would never refresh, since only `refs/remotes/origin` is watched and the watches are
  not recursive — and the fetch age means something only for a remote-tracking ref.
- **Recent.** Git records no time for a working-tree change, so "changed" is either the file's
  mtime or a commit time — see decision 3. The set also *decays*: a file leaves it as time passes
  with no event at all, so something has to re-evaluate it on a timer rather than on an event.
  And mtime alone is cheap per file but not per repository: stat-ing every tracked file on
  each refresh and each tick scales with the repository, not with what changed.
- **Since a commit.** Whether the set includes uncommitted work (`<commit>` against the working
  tree) or only what was committed since (`<commit>..HEAD`). The cheap shape is the committed part,
  cached per `HEAD` as `BranchCache` does, unioned with the dirty files `git status` already
  reports — at the price of marking a file that was changed and then changed back.
- **Custom.** Paths arrive over the socket: root-relative or absolute like `reveal`, resolved to
  real paths, bounded in count and length at the boundary. A path that does not exist yet stays in
  the set and lights up when it appears; a deleted file has no row to mark, as in `078`. It is the
  one mode that needs no git at all.
- **All of them.** Rollup to ancestors as `branch_changes` does, a root annotation that says what
  the set *is* (the user did not necessarily choose it — an agent may have), and the existing
  `branch-badges` surface keeping its meaning.

## Decisions taken (2026-11-16)

Settled before design; the design phase builds on them rather than reopening them.

1. **Look: the `078` column, one producer at a time.** The column keeps its position and width; the
   glyph names the mode (branch mark for a branch diff, a clock for recent, a commit mark for
   since-commit, a pin for custom), and the root annotation names the set —
   `(vs upstream/develop, fetched 2h ago)`, `(changed in the last 30m)`, `(since a1b2c3d)`,
   `(custom: 12 paths)`. Each new glyph needs a Nerd Font codepoint plus a plain fallback measured
   against the five fonts `078` measured. Rejected: a single neutral mark for every mode (an agent
   could swap the set under a user with nothing on the row changing), colouring the file name
   (collides with the git status colour), a row tint (collides with selection and hover).
2. **One active set.** One column, one meaning; setting a highlight replaces the current one.
   Rejected: named layers with a precedence in one column (the mark cannot say which layer put it
   there), a column per layer (width, in the pane the fence is drawn for).
3. **"Changed" in the time-based mode is git-anchored.** A file is recent if it is uncommitted with
   an mtime inside the window, or touched by a commit whose committer date is inside it. The dirty
   files come from the `git status` already run and the committed ones from one `git log --since`,
   and a `checkout` — which rewrites the mtime of every file it touches — does not light up the
   tree. Outside a git repository the mode is unavailable, as the branch column is without a base.
   Rejected: plain mtime of every non-ignored file, which stats the repository on every refresh and
   tick and floods after a branch switch.
4. **Custom is a mode like the others, carrying its list.** There is no `clear`: a custom set ends
   when another mode is set, `off` included. Runtime highlights are never persisted — a set an agent
   chose must not outlive the instance (the reasoning of ADR 0035 for the open command).
5. **One new socket verb, `highlight`, with a `mode`:** `branch` (optional `value`: a ref, or a
   remote name meaning that remote's `HEAD`), `recent` (`value`: a duration), `since` (`value`: a
   commit), `custom` (the list, in a new request field `paths`), and `off`. Client form:
   `birch ctl highlight branch [<ref>] | recent <duration> | since <commit> | custom [<path>…] [--stdin [-0]] | off`.
   A matching `get-highlight` reports the active set, so an agent can check before replacing what a
   user chose. A custom set is replace-only: the caller sends the whole list each time. Rejected: a
   `SettingKey` under `set` with an encoded value (`set` feeds persistence and carries scalars, and
   a path list is neither), a verb per mode (grows the closed set four times over). Likely an ADR:
   the verb, and that runtime highlights are not persisted.
6. **The `branch-badges` surface keeps working** and means "the highlight column is shown". A new
   config key `highlight` and launch flag `--highlight` choose the mode at launch, defaulting to
   today's branch diff against the remote base.

Smaller questions the design phase settles: whether the git-based modes include uncommitted changes
(the branch diff today does not; since-commit arguably should), and whether a directory in a custom
set highlights its whole subtree or only itself.

## Weight

Sprint-sized and design-heavy: the git worker, the watcher, a timer, the protocol and `birch ctl`,
config and flags, the render column, and the root annotation all move. Likely one ADR.

## Related

- [`078-add-branch-diff-badges`](../archive/078-add-branch-diff-badges.md) — what this generalises.
- [`032-add-git-changes-source`](032-add-git-changes-source.md) — the source-shaped neighbour.
- [`099-mark-symlinked-rows`](099-mark-symlinked-rows.md) — also wants a column in the same band.
- ADR 0011 (additive protocol), ADR 0019 (`birch ctl`), ADR 0035 (a runtime setter as its own verb).
