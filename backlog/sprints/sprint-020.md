---
type: Sprint
title: Multi-select and the action surface
status: Designing
branch: sprint/020
tasks:
- 081-standard-navigation-keys
- 088-replace-the-scope-fence
- 074-add-multi-selection
- 071-add-context-menu
---

# Scope rationale

birch can show you anything and do almost nothing with it. Every action task in the backlog waits on
one surface — the context menu — because printable characters are permanently reserved for search
and there are no letter hotkeys to spend. This sprint builds that surface, and settles first what it
acts *on*.

**The fence had to move before any of it.** `docs/design.md` fences scope twice: an in-scope
whitelist and a permanent out-of-scope list, under an identity sentence — *"the IDEA / VS Code
explorer panel"* — that already decides most cases and contradicts the list. The VS Code explorer
has multi-select and drag-to-move; both were permanently out of scope. The maintainer's call is to
delete the out-of-scope list and let the identity be the test.

**Multi-select comes before the menu, deliberately.** It changes what an action operates on. Design
"Delete" for one node and it is designed twice — the confirmation changes, the menu labels change,
and `birch ctl get-path` acquires a question about returning N paths. Settling selection first makes
everything downstream single-pass.

**Half the group was deferred.** The four action tasks that *use* the menu — copy paths, file
operations, open-with, drag-to-move — are much cheaper to design once selection and the menu exist,
so they become sprint 021 rather than doubling this one.

# In-scope task ledger

- **`081-standard-navigation-keys`** — *small, medium.* PgUp/PgDn/Home/End are dead keys. Open for
  design: whether they move the selection or the viewport, and what they mean under a live search,
  where the row set is filtered. Independent of everything else here, so it lands while the fence is
  being decided.
- **`088-replace-the-scope-fence`** — *design-only, medium.* Delete the permanent out-of-scope list;
  settle whether the remaining "In scope (the whitelist)" line is binding or descriptive, since
  multi-select is not on it and removing the blacklist alone would not unblock `074`. Rewrite
  `AGENTS.md`'s reject-by-lookup rule as a judgement test. Needs an ADR: it reverses a stated
  permanent decision and becomes the governing rule for every future scope question. No replacement
  constraints — a carve-out for the plugin API was raised and explicitly declined.
- **`074-add-multi-selection`** — *major, medium.* Shift/Ctrl range and toggle, mouse and keyboard.
  Wider than it looks: the selection model, persisted state (which keys on real paths), rendering,
  `hit_test`, the keyboard map, and the action layer. Meets [ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)
  — dimmed rows are inert and cannot join a selection — and raises an additive socket question for
  `get-path`. Blocked on `088`.
- **`071-add-context-menu`** — *major, medium.* The right-click action surface, split out of the 0.5
  bundle so it can land before the operations it hosts. `AGENTS.md` names it the primary surface for
  actions. Its design depends on `074`: menu entries act on a selection, not a node.

# Ordering / dependencies

- **`081` first** — independent and small, and it does not wait on the fence.
- **`088` before `074`** — it is the gate, and only the maintainer can approve it.
- **`074` before `071`** — the menu acts on a selection; designing it against one node means
  designing it twice.

# Risks

- **The fence is a maintainer decision that gates half the sprint.** If `088` does not land, `074`
  drops and the sprint is `081` plus a context menu acting on a single node.
- **`AGENTS.md` loses a mechanism.** Reject-by-lookup becomes reject-by-judgement. The replacement
  wording has to keep refusing miller columns, a verb language, and vim modes without a list.
- **`074` touches persisted state.** Expansion/selection/scroll are stored per root; a selection that
  is now a set has to restore, or deliberately not restore, without corrupting existing state files.

# Considered but out of scope

- **`028`, `029`, `034`, `072`** — the actions the menu will host. Deferred to sprint 021 by
  agreement; each is cheaper once selection and the menu exist, and `072` additionally needs
  multi-select to drag N and [ADR 0025](../../docs/adr/0025-a-click-completes-on-release.md)'s
  release model so a drag is not read as a click.
- **`083`–`087`** — the cmux adapter queue from the sprint-019 review. Real, none urgent; `084` is
  the only defect with a user-visible cost.
- **`035`** — high priority and still not agent-executable, though `080` makes it worth re-checking
  whether the throwaway-bundle method applies to herdr.
- **`026`, `053`, `070`** and the theme follow-ups (`055`, `056`, `058`, `061`, `065`, `066`) —
  off-theme.
- **`030`, `032`, `033`** — the additional sources; "Later" in the design doc.

# Sprint-start action

Scope committed to `main`; branch `sprint/020` cut from it. Design phase opens with `088`, because it
gates `074`, and with `081` in parallel since it depends on nothing here.

# Checklist

- [ ] 081-standard-navigation-keys — designed
- [ ] 088-replace-the-scope-fence — designed
- [ ] 074-add-multi-selection — designed
- [ ] 071-add-context-menu — designed

# Open questions

_(none open)_

# Session log

- Scoped and cut: `081`, `088`, `074`, `071`. Branch `sprint/020` cut from `main`.
- Design: `088` and `081` presented first, since `088` gates `074`; four questions answered
  (the two-part identity test, per-operation arity, selection-moving page keys, and paging by
  matches under a query). `074` and `071` designed on the settled fence.
- `Shift` with the mouse was **declined on evidence** rather than designed: `docs/design.md` and
  `birch --help` both promise `Shift` to the terminal for native text selection, and ghostty's
  `mouse-shift-capture` defaults to `false`, so the event does not reach the application.
  `.scratchpad/mouse-probe.py` was written to settle the remaining modifier questions in a real
  terminal.
- Two gaps found while designing were filed rather than guessed at:
  `089-keyboard-toggle-for-multi-select` and `090-hover-highlight-in-the-tree`.
- ADRs proposed: 0028 (the identity is the scope fence), 0029 (the menu projects the action layer).
