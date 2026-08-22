---
type: ADR
title: Delete goes to trash, and confirmation appears only where neither trash nor git can help
status: Proposed
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
| trash unavailable | **confirm**, stating plainly that the delete is permanent. Declining does nothing. |

1. **The judgement is over the whole selection**, not per file: one prompt, naming how many rows and
   why, rather than a prompt per target.
2. **A directory is judged by what it contains.** A directory holding any untracked file is treated
   as untracked, because that is what would be lost.
3. **Confirmation is `Enter` / `Esc`** on the status line, never `y`/`n`
   ([ADR 0030](0030-the-inline-editor-is-the-only-mode.md)).
4. **Trash is provided by the `trash` crate**, not by moving files into a folder. The platform
   behaviour is not a rename: macOS records put-back information, and the XDG specification requires
   `.trashinfo` records and a per-mount trash directory with its own collision rules. Reimplementing
   that is a way to lose files quietly.
5. **Trash failure is never silently upgraded to a permanent delete.** When trash is unavailable the
   user is told, and the permanent delete happens only on their confirmation.

# Consequences

- **The common case has no prompt at all.** In a git repository, deleting tracked files is
  immediate — which is the behaviour the git integration was always meant to buy, and the reason it
  doubles as a safety net rather than only a badge.
- **birch depends on `trash`.** On Linux it is `std::fs` over the XDG specification; on macOS a thin
  objc binding. Unlike an in-process clipboard, it links no GUI stack.
- **Deleting is plural.** It is the first action to consume a whole selection, and the first place
  multi-select does something a user could regret — which is why the confirmation reasons about the
  set.
- **There is no undo inside birch.** The trash is the undo, and it lives outside; the status line
  therefore has to say what happened and where it went, because nothing in birch can show it.
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
