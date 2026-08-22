---
type: Sprint
title: The actions the menu was built for
status: Designing
branch: sprint/021
tasks:
- 028-add-copy-paths
- 092-open-the-menu-from-the-keyboard
- 029-add-file-operations
- 034-add-open-with
- 072-drag-to-move
---

# Scope rationale

Sprint 020 built the surface and settled what it acts on: a selection that can hold several rows,
and a context menu generated from an action layer where adding an action *is* the whole of adding an
entry. What it could not build was anything to put there. The menu ships with `Open` and
`Reveal Root Here` because those are the two actions that exist; seven of the nine entries
[`docs/design.md`](../../docs/design.md) specifies are still absent.

This sprint fills them. Every task in it was waiting on the menu, and each now costs an action
declaration plus its implementation rather than a surface design — which is the whole reason
[ADR 0029](../../docs/adr/0029-the-menu-projects-the-action-layer.md) made the menu a projection.

It is also the sprint where birch starts **mutating files**. Everything before it read the tree,
rendered it, and reported paths. Four operations and a drag gesture change that, and the care in the
design phase belongs there: the op layer owning watcher events during a mutation, delete choosing
trash over confirmation, and a drag that cannot be mistaken for a click.

# In-scope task ledger

- **`028-add-copy-paths`** — *medium, mid.* The copy primitive — name, root-relative path, absolute
  path — over the OSC 52-first clipboard chain (which works over SSH and inside tmux), with
  `pbcopy`/`wl-copy`/`xclip` as native fallback and the status line as last resort. Three menu
  entries and the `Ctrl-Shift-C` accelerator. Open for design: what a failed copy says, since OSC 52
  fails silently by nature — a clipboard that quietly did nothing is worse than one that refused.
- **`092-open-the-menu-from-the-keyboard`** — *medium, mid.* Added to scope after the sprint opened.
  The menu has exactly one trigger — the right button — so a host that consumes right-click before it
  reaches birch makes the menu unreachable there entirely, and there is no keyboard way in at all.
  That gap costs little today, when the menu holds two entries; by the end of this sprint it hides
  Copy, New File, Rename, and Delete. Open for design: which key (printables are reserved, so
  `Ctrl-Space`, `Shift-F10`, and the `Menu` key are the candidates), where the menu appears when no
  pointer placed it, and whether a keyboard trigger disturbs the selection — it should not.
- **`029-add-file-operations`** — *medium, design-heavy.* The op layer: rename, delete-to-trash, new
  file, new dir, as **inline row editing** rather than a prompt bar (prompt bars grow verbs).
  Git-aware delete — a tracked file is nearly free and recoverable, an untracked one gets friction
  and is told why. Plus the watcher-event ownership that keeps the tree from flickering or jumping
  mid-edit. Open: trash on exotic filesystems and NFS, and `F2` on a compacted chain editing the
  full `a/b/c` fragment.
- **`034-add-open-with`** — *low, design-heavy but a small build.* One menu entry offering
  alternative open commands. Cheap once the menu and the config file
  ([ADR 0022](../../docs/adr/0022-config-file.md)) exist; the whole task is one product decision —
  where the alternatives come from.
- **`072-drag-to-move`** — *medium, design-heavy.* Drag a row onto a directory to move it.
  [ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md) turned "no move" from a
  permanent prohibition into the File operations section's own decision, so this now needs an
  argument and an ADR rather than a fence amendment. Multi-select and
  [ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md)'s release model — the two things
  sprint 020's scope named as its prerequisites — both exist now.

# Ordering / dependencies

- **`028` first** — it needs nothing from the op layer, and it exercises the new action layer end to
  end with an action that cannot destroy anything.
- **`092` early, and independent of the rest** — it touches the menu's trigger, not its contents, so
  it neither waits on the op layer nor blocks it. Landing it early means every action this sprint
  adds is reachable from the keyboard the day it arrives, rather than retrofitted at the end.
- **`029` next** — it is the op layer the two remaining tasks ride on.
- **`034` and `072` after `029`** — `034` adds an entry beside `Open`; `072` needs the mutation path
  and the watcher ownership `029` builds.

# Risks

- **`029` is the largest single task in the backlog.** Inline row editing is a new input mode in a
  product whose identity refuses modes; trash is platform-specific with a real fallback question;
  and watcher-event ownership during a mutation is the subtle part that decides whether the tree
  flickers or the selection jumps while a name is being typed.
- **`072` is a second design-heavy task with its own ADR**, and it cannot start until `029` lands.
  Its hard parts are named in the task: separating a drag from a click under mouse capture,
  spring-loading a collapsed directory against the standing "never auto-expand" rule, illegal and
  colliding drops, and the fact that a move has no undo — there is no op-history stack and trash
  cannot help.
- **`034` is one decision away from trivial** and stalls completely until that decision is made.
  Worth settling early in the design phase rather than late.
- **This sprint writes to the user's filesystem.** Every previous sprint could be wrong on screen;
  this one can be wrong on disk.

# Considered but out of scope

- **`035-verify-herdr-integration`** — the only `high` in the backlog, deferred to the next sprint by
  maintainer decision. It needs a live herdr session, and sprint 019 showed that a task scoped as
  "probably needs the maintainer" can turn out automatable, so whether the throwaway-instance method
  transfers is worth re-checking when it is scheduled.
- **`083`–`087`** — the cmux adapter queue. `084` is a real leak (the follow watcher is not signalled
  when the window closes, so it forks a `cmux` call and a `python3` on every workspace switch,
  forever) and `085` means a failing `--open-cmd` says nothing at all. Both are defects rather than
  features; deferred as off-theme, not as unimportant.
- **`073-hotkey-reference`** — worth more than it was, since sprints 020 and 021 add
  `PgUp`/`PgDn`/`Home`/`End`, the `Shift` variants, and a whole menu layer. But it is a
  discoverability task, not an action task.
- **`089`, `090`** — filed during sprint 020, both low, both genuinely optional.
- **`026`, `030`, `032`, `033`, `053`** and the theme follow-ups (`055`, `056`, `058`, `061`, `065`,
  `066`) — off-theme.

# Sprint-start action

Scope committed to `main`; branch `sprint/021` cut from it. The design phase opens with `028`, which
depends on nothing here, and `029`, which the rest of the sprint waits on.

**Scope amended after the sprint opened**, by maintainer decision:
`092-open-the-menu-from-the-keyboard` was added once it became clear the menu has no keyboard
trigger at all — a gap that grows with every action this sprint adds to it.

# Checklist

- [ ] 028-add-copy-paths
- [ ] 092-open-the-menu-from-the-keyboard
- [ ] 029-add-file-operations
- [ ] 034-add-open-with
- [ ] 072-drag-to-move

# Open questions

_(none yet)_
