---
type: ADR
title: Delete goes to trash, and confirmation appears only where neither trash nor git can help
status: Accepted
sprint: sprint-021
---

# Context

`docs/design.md` states the principle — **undoable beats confirmable** — and the shape: delete to the
platform trash, with *"confirmation only where trash can't help"*, and **git-aware**, since deleting
a tracked file is nearly free while an untracked one exists nowhere else.

Turning that into behaviour needs the cases enumerated, because there are **two** safety nets rather
than one and they do not cover the same ground. Trash is undoable but can be emptied. Git is durable
but only covers what is committed. A rule that names only one of them either over-confirms the safe
case or under-confirms the unrecoverable one.

Deleting is also the first operation that acts on a whole selection
([`074`](../../backlog/archive/074-add-multi-selection.md) left arity to each operation), so the
question is asked of a set, not a file.

# Decision

**Delete moves to the platform trash. Confirmation appears exactly where neither trash nor git
reaches.**

| situation | behaviour |
|---|---|
| trash works · every target tracked in git | **delete immediately**, no prompt. Recoverable twice over; a confirmation here is theatre, and theatre is what trains people to dismiss prompts unread. The status line reports what went. |
| trash works · any target untracked | **confirm once**, naming the count and the reason. An untracked file exists nowhere else, and a trash can be emptied. |
| trash unavailable | **nothing is deleted.** The failure is reported and the delete ends there. |

1. **The judgement is over the whole selection**, not per file: one prompt, naming how many rows and
   why, rather than a prompt per target.
2. **A directory is judged by what it contains.** A directory holding any untracked file is treated
   as untracked, because that is what would be lost.
3. **Confirmation is `Enter` / `Esc`** on the status line, never `y`/`n`
   ([ADR 0030](0030-the-inline-editor-is-the-only-mode.md)).
4. **Trash is provided by the `trash` crate**, not by moving files into a folder. The platform
   behaviour is not a rename: the XDG specification requires `.trashinfo` records and a per-mount
   trash directory with its own collision rules. Reimplementing that is a way to lose files quietly.

   *Amended 2026-08-22:* **on macOS birch selects `DeleteMethod::NsFileManager`**, not the crate's
   `Finder` default. Finder's method runs `osascript -e 'tell application "Finder" to delete …'`,
   which plays Finder's delete sound on every call and needs Automation permission. That permission
   can be **denied**, and a denial surfaces as a trash failure — pushing the user into decision 5's
   "the trash could not take it" failure for a reason that has nothing to do with the trash — and
   since decision 5's amendment, that failure is the end of the delete rather than a prompt. The cost is Finder's **Put Back**: a trashed file still sits in the
   trash and can be dragged out, but no longer remembers where it came from. This paragraph
   originally cited put-back as a reason to prefer the crate; that reason is withdrawn — birch now
   provides the restore itself, through [ADR 0033](0033-undo-is-a-session-stack.md)'s undo stack,
   which does not depend on Finder remembering anything. The XDG half stands.
5. **There is no delete that bypasses the trash** — not as a fallback, not as a deliberate choice.
   When the trash cannot take a path, birch reports the failure and deletes nothing.

   *Amended 2026-08-22.* This decision originally offered a permanent delete on confirmation when
   the trash was unavailable, on the reasoning that a user who asked to delete should get an answer
   rather than a dead end. That was the wrong trade. The whole of this ADR rests on delete being
   **recoverable** — that is what buys the no-prompt fast path for tracked files, and what makes one
   prompt enough for untracked ones. A route that removes the recoverability removes the foundation,
   and it was reachable exactly when birch understood the situation *least*: the trash had already
   failed for a reason birch could not explain. A failure is a better answer than an irreversible
   one. `ops::delete_permanently` is deleted rather than left unreachable, so nothing can wire a
   path back to it by accident.

# Consequences

- **The common case has no prompt at all.** In a git repository, deleting tracked files is
  immediate — which is the behaviour the git integration was always meant to buy, and the reason it
  doubles as a safety net rather than only a badge.
- **birch depends on `trash`.** On Linux it is `std::fs` over the XDG specification; on macOS a thin
  objc binding. Unlike an in-process clipboard, it links no GUI stack.
- **Deleting is plural.** It is the first action to consume a whole selection, and the first place
  multi-select does something a user could regret — which is why the confirmation reasons about the
  set.
- **There is no undo inside birch *for delete*.** The trash is the undo, and it lives outside; the
  status line therefore has to say what happened and where it went, because nothing in birch can
  show it. Amended by [ADR 0033](0033-undo-is-a-session-stack.md): birch gained an undo stack, and
  delete is the one step on it that cannot be reversed — `trash`'s `restore_all` is not compiled on
  macOS — so `Ctrl-Z` after a delete repeats where the files went rather than reversing something
  else.
- **A non-git directory always confirms.** Everything there is untracked by definition. That is
  correct rather than annoying, but it is the case a user meets most often outside a repository.

# Alternatives considered

- **Always confirm.** Rejected: it trains dismissal. A prompt that is right to ignore nine times out
  of ten is worse than no prompt, because it is ignored the tenth.
- **Never confirm, trust the trash.** Rejected: a trash that has been emptied, or a filesystem that
  has none, turns "undoable" into "gone" with no signal.
- **Permanent delete when trash is unavailable, silently.** Rejected outright. The whole promise is
  that delete is recoverable; performing an unrecoverable one under the same keystroke, without
  saying so, is the worst available behaviour.
- **An in-birch undo stack.** Rejected by `docs/design.md` before this task, and it would not help:
  undo across a process restart means a journal, and a journal of file mutations is a different
  product.
