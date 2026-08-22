---
type: ADR
title: The identity is the scope fence; the permanent out-of-scope list is deleted
status: Proposed
sprint: sprint-020
---

# Context

`docs/design.md` fenced scope three times over. An identity sentence —

> birch does exactly what an IDE file tree does, and nothing else. It is the IDEA / VS Code explorer
> panel, living in a terminal pane next to your editor.

— followed by an **in-scope whitelist** and a **permanently out of scope** list. The sentence called
itself "both the feature list and the scope fence", and then two lists were laid over it.

The three disagree. The VS Code explorer panel has Ctrl/Shift multi-select and drag-to-move; both
sat on the permanent list, together with bulk operations, which follows from multi-select the moment
three selected files are deleted. So the lists forbade what the identity includes, and a request for
multi-select — raised by the maintainer — could be answered either "yes, that is what the panel
does" or "no, it is permanently out of scope", depending on which line was read.

The lists also duplicated work already done elsewhere. Nearly every entry restates a prohibition
that some section of `docs/design.md` makes for itself, with a reason attached: no plugins
(architecture, in Integrations), no search-and-replace (Content search), no copy and no move (File
operations), no verbs and no modes (the `Not broot` contrast line, and
[ADR 0008](0008-q-types-into-search.md), under which printable characters are search input and a
letter hotkey cannot exist). The list held those decisions as bare nouns, stripped of the reasoning
that made them right.

`AGENTS.md` turned the list into a procedure: *"Scope is a whitelist. Reject features on the
out-of-scope list."* Rejection by lookup is cheap and needs no judgement, which is its appeal and
its defect — it cannot tell a feature that contradicts the product from one that merely was not
foreseen, and it answers a maintainer request with a citation.

# Decision

**The identity sentence is the whole fence. The permanent out-of-scope list is deleted, and the
in-scope line becomes descriptive.**

1. **The test is the identity, and it has two halves — both must hold.**
   - **The panel's job.** Does the IDEA / VS Code explorer panel do this? The panel is a tree of
     files plus the operations reached from a row: not a file manager, not a diff viewer, not a
     shell, not a second navigation model.
   - **The pane's medium.** Does it survive a narrow terminal pane where every printable character
     belongs to search (ADR 0008)? A feature needing width, a second axis, a modal state, or a
     letter hotkey fails here however ordinary it is inside an IDE.

   The second half is not new wording; it is the clause that was already in the sentence. Naming it
   is what stops "the identity is the test" from becoming "anything an IDE does".

2. **The permanent out-of-scope paragraph is deleted.** Nothing replaces it. A carve-out preserving
   the plugin prohibition as a scope rule was considered and declined: that prohibition is argued
   from architecture in the Integrations section and stands there on its own.

3. **The in-scope line is a description, not a gate.** It summarises what birch does today and
   grows as features ship. It admits nothing and refuses nothing, and the word "whitelist" leaves
   it. A feature absent from the line is not thereby out of scope — the identity decides.

4. **Prohibitions that carry their own reasoning stay where they are, unchanged**: no plugins, no
   search-and-replace, no mutation verbs on the control socket, no op-history, no copy, no move.
   Each is now argued in exactly one place. The socket's verb set remains closed for its own reason
   — it is a public API — and no longer by reference to a scope whitelist.

5. **Arity is per operation, not a global rule.** `no multi-select` leaves the File operations
   opener. Whether an operation accepts one target or several is decided when that operation is
   designed.

6. **`AGENTS.md` states the test instead of the list.** An agent admits a feature only if both
   halves hold, refuses a different *shape* whichever neighbouring tool has it, honours the
   section-local prohibitions on their own terms, and takes unclear cases to the maintainer.
   "Another file manager has it" is not an argument.

# Consequences

- **Multi-select is admitted**, which unblocks `074-add-multi-selection`. It is the one entry the
  deleted list was holding by itself.
- **Drag-to-move becomes arguable** rather than settled. `072-drag-to-move` is now judged against
  the File operations section's own `No move` and the rename-to-a-path reasoning under it — a
  weaker prohibition than "permanent", deliberately.
- **Refusal costs more than it did.** Rejection by lookup is gone; a refusal now has to name which
  half of the identity fails and why. This is the point of the change, and its price: the argument
  is slower, and a wrong one is easier to make. Where a section already states a prohibition, cite
  the section, not the identity.
- **What was refused stays refused.** Miller columns fail the panel half (a tree is not columns); a
  verb language and vim modes fail the pane half twice over, by ADR 0008 and by the modal-state
  clause; archive browsing and permissions editing are not the panel's job. The list is not needed
  to reach any of those.
- **The identity sentence becomes load-bearing text.** Editing it edits the scope fence. It is now
  the one place a scope question is answered from, so a change to it is an ADR-weight change.
- **Nothing in the product changes.** No flag, key, protocol field, or on-disk format moves; this
  decision governs what may be proposed, not what birch does.

# Alternatives considered

- **Delete the blacklist and leave the whitelist binding.** Rejected: multi-select is not on the
  whitelist either, so this would not have unblocked the request that prompted the change, and a
  whitelist extended one entry per admitted feature is exactly the mechanism that produced the
  contradiction.
- **Keep the blacklist and strike only multi-select and drag-and-drop move from it.** Rejected: it
  leaves a list whose remaining entries duplicate reasoned prohibitions elsewhere, and it settles
  one request while preserving the machine that will misanswer the next.
- **Replace the list with a smaller set of permanent constraints** — a plugin API carve-out was the
  concrete proposal. Declined by the maintainer. The anti-plugin instinct survives where it was
  always argued from architecture rather than scope.
- **Adopt the identity with no second half** — "would an IDE explorer panel do this?" alone.
  Rejected: the VS Code panel offers `Compare Selected`, breadcrumbs, an integrated terminal, and
  cut/copy/paste of files. The medium half is what refuses those without a list, and it was already
  written into the sentence.
