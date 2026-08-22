---
type: Task
title: The identity is the scope fence
description: Delete the permanently-out-of-scope list and let "what an IDE file tree does" be the whole test.
status: Draft
priority: medium
---

[`docs/design.md:7-13`](../../docs/design.md) fences scope twice — an in-scope whitelist and a
permanent out-of-scope list — under an identity sentence that already decides most cases:

> birch does exactly what an IDE file tree does, and nothing else. It is the IDEA / VS Code explorer
> panel, living in a terminal pane next to your editor.

The two disagree. The VS Code explorer has Ctrl/Shift multi-select and drag-to-move; both are on the
permanent list, along with bulk operations, which follows from multi-select the moment you delete
three selected files. So the list forbids things the identity includes.

The maintainer's decision: **delete the out-of-scope list.** The identity is the test. No
replacement constraints — including for the plugin API, which was raised and explicitly declined.
(The anti-plugin instinct survives incidentally where it was always argued from architecture rather
than scope: "birch stays free of N host integrations — same instinct as no plugins", in the adapters
section.)

## The part that needs deciding

Removing only the list leaves line 11 — **"In scope (the whitelist)"** — still binding, and
**multi-select is not on it**. Deleting the blacklist alone would therefore not unblock
[`074`](074-add-multi-selection.md) at all.

So this task has to settle what that line becomes:

- **descriptive** — a summary of what birch does today, with the identity sentence as the only test; or
- **binding but extended** — a whitelist that grows an entry per admitted feature, which is what
  produced the contradiction in the first place.

The maintainer's framing ("keep the IDE file tree experience target") points at the first. Either
way, say so explicitly, because "whitelist" is a load-bearing word — `AGENTS.md` currently instructs
agents to reject by lookup.

## What changes

- **[`docs/design.md`](../../docs/design.md)** — the out-of-scope paragraph goes; line 11's status is
  settled.
- **[`AGENTS.md`](../../AGENTS.md)** — "Scope is a whitelist. Reject features on the out-of-scope
  list in `docs/design.md` (multi-select, verbs/modes, plugins, search-and-replace, drag-and-drop,
  copy/move files, …)" becomes a judgement test against the identity. This is a real loss of
  mechanism: an agent can no longer refuse a feature by lookup, and must argue from "would an IDE
  explorer panel do this?". Write the replacement so that it still refuses miller columns, a verb
  language, and vim modes — which the identity does exclude — without needing a list.
- **An ADR**, because this is a reversal of a stated permanent decision and the governing rule for
  every future scope question.
- **`docs/design.md:238`** — the socket protocol's own verb whitelist is a *different* closed set and
  stays closed. Check the wording there does not lean on the paragraph being deleted.

## Gates

Two tasks in this sprint are blocked until this lands: [`074`](074-add-multi-selection.md) and, in a
later sprint, [`072`](072-drag-to-move.md).

## Design

Design-only — no code. Three documents change (`docs/design.md`, `AGENTS.md`, a new ADR) and one
backlog gate opens.

### The fence becomes a two-part test, read out of the sentence already there

> birch does exactly what an IDE file tree does, and nothing else. It is the IDEA / VS Code explorer
> panel, living in a terminal pane next to your editor.

The sentence has two halves, and the fence is **both of them, jointly**:

1. **The panel's job.** Does the IDEA or VS Code explorer panel do this? The panel is a tree of
   files plus the operations reached from a row. It is not a file manager, not a diff viewer, not a
   shell, and not a second navigation model.
2. **The pane's medium.** Does it survive a narrow terminal pane in which every printable character
   belongs to search ([ADR 0008](../../docs/adr/0008-q-types-into-search.md))? A feature that needs
   width, a second axis, a modal state, or a letter hotkey fails here however ordinary it is inside
   an IDE.

Nothing is added: the second half is the clause that was always in the sentence. Naming it is what
keeps "the identity is the test" from collapsing into "anything an IDE does" — the panel-half alone
would admit `Compare Selected`, breadcrumbs, and a built-in terminal.

### What the deleted list was actually holding

Ten of its eleven entries are refused by rules that survive the deletion; the list was an index of
decisions stated elsewhere with their reasons, not the decisions themselves.

| deleted entry | what refuses it once the list is gone |
|---|---|
| miller columns | the identity — a tree, not columns; and the `Not yazi` contrast line |
| verbs or command language | ADR 0008 (printables are search) and `Not broot — no verbs, no modal behavior` |
| vim modes | the same two |
| search-and-replace | Content search: *"No search-and-replace, ever"* |
| plugin API | Integrations: *"No plugins in birch"* — argued from architecture, not scope |
| copy/duplicate files | File operations: *"no copy"* |
| drag-and-drop move | File operations: *"No move"* — [`072`](072-drag-to-move.md) reopens it there, on its own merits |
| bulk operations | File operations owns per-operation arity; [`074`](074-add-multi-selection.md) settles it |
| archive browsing | judgement: not the panel's job |
| permissions editing | judgement: not the panel's job |
| **multi-select** | **nothing — this is the entry the deletion admits** |

The loss of mechanism is therefore narrower than it looks. What goes is the ability to refuse by
lookup; what stays is every prohibition that carried a reason, in the section that owns it.

### Edits

**[`docs/design.md`](../../docs/design.md)**

- **Line 11** keeps its list and loses its authority. `**In scope (the whitelist):**` becomes a
  description of the present surface, with an explicit statement that it does not gate anything and
  that the identity above is the only gate. The list is not extended with multi-select now: it
  describes what birch does, and [`074`](074-add-multi-selection.md) adds the word when the feature
  ships.
- **Line 13** — the `Permanently out of scope:` paragraph is deleted outright.
- **Line 177** — `no multi-select` is struck from the File operations opener. Whether an operation
  accepts several targets is that operation's question, and belongs to
  [`074`](074-add-multi-selection.md) and [`029`](029-add-file-operations.md); the rest of the
  sentence (`No move (see below), no copy, no op-history/undo stack`) stands unchanged.
- **Line 238** — the socket verb table's `(closed set — the whitelist applies to the protocol too)`
  loses its reference to the deleted paragraph and states its own reason: the protocol is a public
  API and stays small. The set stays closed.

**[`AGENTS.md`](../../AGENTS.md)** — the `Scope is a whitelist` bullet is replaced by the judgement
test: admit a feature only if the explorer panel has it **and** it survives a narrow terminal pane
where printables belong to search; a different *shape* fails whichever neighbouring tool has it;
prohibitions stated by a section of `docs/design.md` stand on their own terms; unclear cases go to
the maintainer, and "another file manager has it" is not an argument.

**[ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md)** — `Proposed` with this task.
It reverses a decision stated as permanent and becomes the rule every later scope question is
decided by, so the reversal needs a record of its own.

### Public-surface delta

**None.** No CLI flag, config key, socket field, environment variable, on-disk path, or library API
changes. The change is to the governing documents and to what a future task is allowed to propose.

### Gate released

[`074-add-multi-selection`](074-add-multi-selection.md) becomes designable in this sprint;
[`072-drag-to-move`](072-drag-to-move.md) becomes arguable in a later one against the File
operations section's own `No move`, not against a permanent list.
