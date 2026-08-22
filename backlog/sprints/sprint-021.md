---
type: Sprint
title: The actions the menu was built for
status: Done
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
- **`093-undo-a-file-operation`** — filed during this sprint's design phase, and deliberately not
  scoped into it. Once birch mutates, only delete has a way back (the trash); rename, move, and
  create have none. Reversing `docs/design.md`'s "no op-history/undo stack" is its own argument with
  its own ADR, and bolting it onto a sprint that is already five tasks would rush it.
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

Branch `sprint/021`, base `main`. **Nine implementation commits** on top of the approved design
merge; the rest of the branch is close-out bookkeeping. 328 tests pass, up from 316 before the
review round; `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.

**This is the sprint where birch began writing to the user's filesystem.** Everything before it read
the tree, painted it, and reported paths; a wrong answer was wrong on screen. From here a wrong
answer is wrong on disk, and both the design and the review were weighted accordingly.

# Task ledger

Weight is size **and** importance **and** future impact together.

| task | weight | what it is |
|---|---|---|
| `029-add-file-operations` | **major** | The op layer and the inline editor: `F2` renames, `Ctrl-N` creates (a trailing slash makes a directory, and intermediates come with it), `Delete` moves the selection to the platform trash. Renaming to a path moves. Delete asks only where neither trash nor git reaches. Delivered as designed; the watcher-ownership worry the task carried dissolved on contact with ADR 0006, since a snapshot reconciles rather than appends. |
| `072-drag-to-move` | **major** | Drag a row onto a directory to move it. A drag is a click that left its row, so ADR 0025's model already tracks it with no threshold. Delivered as designed — and spring-loading turned out to need no carve-out, because the never-auto-expand rule is scoped to *ignored* directories and search already auto-expands. |
| `028-add-copy-paths` | **mid** | Copy name, root-relative path, or absolute path, newline-joined in tree order for several rows. One clipboard transport per copy, chosen the way Neovim chooses it. Two changes from what `docs/design.md` sketched: the three-tier fallback cannot be built, because OSC 52 is a one-way write with no failure to react to; and `Ctrl-Shift-C` is **dropped**, being the same byte as `Ctrl-C`, which quits. |
| `092-open-the-menu-from-the-keyboard` | **mid** | `Shift-F10` and the `Menu` key open the menu at the cursor row without disturbing the selection. **Added to scope after the sprint opened**, once it was clear the menu had exactly one trigger and a host that swallows right-click made it unreachable. |
| `034-add-open-with` | **minor** | `Open` names the program it will run, and `Open system` hands the path to the desktop's handler — absent when the resolved command already is that handler, on a multi-row selection, and in `--pick`. Rewritten during design from the task's "Open with…" framing: the config array and the submenu were both dropped in favour of two rows and **no config surface at all**. |

All five were delivered; none was dropped or reduced.

# Breaking changes

**None.** No output, wire format, or on-disk format changed — `protocol.rs` is untouched, `--pick`
output is unchanged, and no persisted format moved. Five keys that were previously **unbound** gain
meanings: `F2`, `Ctrl-N`, `Delete`, `Shift-F10`, and the `Menu` key. `Esc` gains two layers, backing
out of a confirmation and an edit before the menu, the search, and quitting.

The documented `Ctrl-Shift-C` accelerator is **removed from the design doc**, having never worked:
in the key encoding birch decodes with, `Shift` has no representation alongside a control character.

# Backlog changes

Filed during the sprint and deliberately not scoped into it. Informational; no approval is asked.

| task | why it exists |
|---|---|
| `091-kitty-keyboard-protocol` (`Draft`) | The legacy encoding cannot represent `Shift` with a control character — nor `Cmd` at all — so `Ctrl-Shift-C` and every `Cmd` chord are unreachable. Adopting the Kitty protocol changes how *every* key is decoded, so it is its own task. |
| `093-undo-a-file-operation` (`Draft`) | After this sprint only delete has a way back, through the trash. Rename, move, and create have none. Reversing `docs/design.md`'s "no op-history/undo stack" is its own argument with its own ADR. |

# Architectural decisions

- **ADR 0030** — the inline editor is birch's only mode. ADR 0008 is read as it was always meant: printables go to whatever text field is focused, and an edit focuses a different one, so no letter becomes a command. The editor is a paint-time overlay that never touches the tree, so an abandoned edit leaves nothing behind and no watcher suppression is needed. Confirmations answer to `Enter` and `Esc`, never `y`/`n`.
- **ADR 0031** — delete goes to trash, and confirmation appears only where neither trash nor git reaches. Tracked paths delete immediately, untracked confirm once, an unavailable trash confirms saying the delete is permanent, and a trash failure is never silently upgraded.
- **ADR 0032** — files move by dragging, and rename-to-a-path stays as the keyboard route. The original "no move" argument was right about capability and wrong about discovery: nothing on screen suggests a name field accepts `../`, and it cannot move a multi-selection at all.

# Bugs found and fixed

**Fourteen, all from the independent review** — a fresh reviewer with no implementation context,
reading the whole diff and running the gates itself (`a8a7404`). Three risked data:

1. **Dragging moved files in `--pick`**, the one place mutations are promised impossible. The gesture bypasses `Action::available`, so nothing enforced it; a script using `birch --pick` as a chooser could rearrange the tree if the pointer wandered.
2. **The delete safety gate had two holes**, both toward less friction. An ignored path is never in git's status map, and is exactly what git cannot bring back. And `dir_status` is a most-severe-wins rollup, so a folder holding one modified tracked file masked every untracked file beside it — selecting such a folder and pressing `Delete` skipped the prompt entirely.
3. **Renaming a compacted chain pre-filled the row's label** (`a/b/c`) while the editor's path is the tail, so committing resolved against the tail's parent — usually a baffling error, and where the intermediate directory existed, a directory moved somewhere nobody asked for.

And a fourth of a different kind: **spring-loading and edge auto-scroll were documented as shipped
and were absent from the binary.** A patch script aborted on an assertion before writing and the
loss went unnoticed; an audit of that batch then found a second silent no-op, which is how finding 4
below came to be fixed rather than shipped.

4. A multi-row drag could not see collisions **inside its own set**, so two selected rows sharing a basename both passed and the move half-applied.
5. `ops::trash` and `delete_permanently` discarded the deltas for paths they had already removed, so a partial removal left the tree showing rows whose files were gone — and then offered a permanent-delete prompt over paths already in the trash, which would have deleted nothing.
6. A **case-only rename was impossible** on macOS: `exists()` resolves case-insensitively, so `readme.md` → `README.md` read as a collision with itself.
7. `exists()` also follows symlinks, so a **dangling link at a destination was silently replaced**, contradicting "nothing here overwrites".
8. `open_is_platform` compared only the program name, so `$VISUAL="open -a Preview"` hid `Open system`.
9. A `create` could **climb out of its directory** and write a file no row could ever show.
10. A trailing space hid a trailing slash, giving a file where a directory was asked for.
11. A phantom row one line past the viewport left the editor **open and invisible** — exactly what ADR 0030 admits the mode for not being.
12. A whitespace-only name was dropped in silence.
13. Native clipboard children were never reaped, so `pbcopy` and `wl-copy` piled up as zombies. Waiting inline would hang on `xclip`, which stays resident to serve the X11 selection, so a thread does the waiting.
14. Two dead parameters kept only to satisfy a signature.

# Verification

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, 328 tests (316 before the
review round). The reviewer's list of uncovered branches is closed — the delete safety gate had no
test at all, and neither did the picker refusal, the partial removal, the within-set collision,
spring-loading, or the `Esc` layering order.

Hands-on, all run and observed:

- **`028`** — a sentinel placed in the real clipboard first, then copying a two-row selection from the menu left `src\na.txt` in it, with OSC 52 correctly unused on a local session that has `pbcopy`.
- **`092`** — `Shift-F10` in a pty opened a layer, proven by birch surviving the following `Esc`, with the selection undisturbed.
- **`029`** — on disk: `Ctrl-N` made nested intermediates and a trailing-slash directory, `F2` renamed, and `Delete` asked before acting — `Esc` left the file alone, `Enter` moved it to the trash.
- **`034`** — a scripted open command wrote a marker file when the menu's first row was invoked. `Open system` is **not** driven: it would launch a real application on the machine running the check.
- **`072`** — real SGR press, drag and release moved `a.txt` into `sub`; a second drag onto a directory already holding that name was refused with the file left where it was.
- **`vhs` frames** of the two new visual states, which caught two things the driven checks could not: the inline edit field sat two columns left of every name around it, and the delete confirmation overran an 80-column pane entirely. The prompt was shortened and front-loaded — but at roughly 50 columns it **still truncates in the 30-column pane birch is designed for**, which no wording fixes: a file name, a question, and a key do not fit. What survives the cut is the question, which is the half that matters.

# Remaining limitations and highlights

- **birch now writes to the filesystem, and there is no undo.** The trash is delete's way back and it lives outside birch; rename, move, and create have none. `093-undo-a-file-operation` holds this.
- **On an Apple keyboard the key labelled *Delete* sends Backspace**, which belongs to the search, so deleting is `Fn`-`Delete`. Binding Backspace to a destructive action whose meaning depended on whether a search was open would be a contextual hazard; the menu is the discoverable path.
- **`Shift-F10` needs `Fn` on a Mac** unless standard function keys are enabled, and the `Menu` key does not exist on Apple keyboards. macOS has no contextual-menu shortcut of its own to match instead, and no `Ctrl`+letter was spent — that space is the scarcest birch has.
- **Edge auto-scroll during a drag needs the pointer to keep moving.** Holding still at the pane's edge does not keep scrolling, because motion events only arrive when the pointer moves; a timer would be machinery this event loop needs for nothing else.
- **A drop refuses rather than offering a new name.** `029`'s editor makes that possible later; a drop that sometimes opens a text field is a surprising drop.
- **Which mouse modifiers arrive is the terminal's decision.** ghostty's `mouse-shift-capture` defaults to `false`, so `Shift`-click never reaches birch there. Nothing in the behaviour depends on the answer.
- **birch gained a runtime dependency**, `trash`, which pulls `objc2`/`windows` bindings into the lockfile. It is new supply-chain surface in `birch-core` — the crate whose whole discipline is building without ratatui — accepted because the platform trash is not a rename into a folder and reimplementing the XDG spec is a way to lose files quietly (ADR 0031).
- **`Delete` now acts while a search is open.** It deletes the selected match, where forward-`Delete` was previously inert. That follows from `Delete` acting on the selection, but it is a destructive key that used to do nothing.
- **A very large copy may be truncated.** Terminals and tmux cap OSC 52 payloads, and birch neither chunks nor warns — consistent with the design's position that nothing here can know a copy failed.

# Session log

- Scoped and cut: `028`, `029`, `034`, `072`. Branch `sprint/021` cut from `main`.
- **Scope amended after opening**: `092-open-the-menu-from-the-keyboard` added by maintainer
  decision, once it was clear the menu had exactly one trigger.
- Design: five questions answered — the clipboard transport (settled on Neovim's shape after
  surveying yazi, Helix, nnn and broot), dropping `Ctrl-Shift-C`, the menu's keyboard binding,
  `034` rewritten to two rows with no config surface, and the scope amendment. ADRs 0030, 0031
  and 0032 proposed and accepted.
- Implementation: `028` and `092` first, being independent of the op layer; then `029`, which the
  rest waited on; then `034` and `072`.
- Close-out: gates green (fmt, clippy, 328 tests, hands-on verification of all five tasks,
  publication hygiene). The independent review found **fourteen** defects, three of which risked
  data, and one feature documented as shipped that was absent from the binary because a patch
  script aborted before writing. All fixed on the branch.

# Checklist

- [x] 028-add-copy-paths — implemented, gates green, verified against a real clipboard
- [x] 092-open-the-menu-from-the-keyboard — implemented, gates green, verified in a pty
- [x] 029-add-file-operations — implemented, gates green, verified on disk and with vhs
- [x] 034-add-open-with — implemented, gates green, configured command verified
- [x] 072-drag-to-move — implemented, gates green, verified on disk

# Open questions

_(none yet)_
