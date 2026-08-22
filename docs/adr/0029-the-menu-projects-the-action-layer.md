---
type: ADR
title: The context menu is a projection of the action layer, and an unavailable action is absent
status: Proposed
sprint: sprint-020
---

# Context

`AGENTS.md` states the rule already: *"Actions live in one action layer consumed by hotkeys, mouse,
context menu, and socket alike. Menu-specific logic is a smell."* No such layer exists. What birch
has instead is `NavEffect` — a small set of outcomes navigation can produce — plus methods on the
app reached directly from `handle_input` and from the socket's verb match. Nothing is wrong with
that while the only actions are open, expand, and re-root; it stops working the moment a second
surface has to ask *"can this be done to these rows right now?"*.

The context menu is that second surface, and four backlog tasks queue behind it — `028-add-copy-paths`,
`029-add-file-operations`, `034-add-open-with`, `072-drag-to-move` — each of which would otherwise
add a menu entry, a hotkey path, and an enablement rule in three places. `docs/design.md` specifies
an eleven-entry menu; birch can implement two of those entries today.

So two questions have to be answered together: what shape an action has, and what the menu shows for
an action that has not been built yet.

# Decision

**The menu does not have contents. It has a source, and it draws whatever that source currently
offers.**

1. **An action is a declaration, in `birch-core`.** Each carries an identity, a display label, an
   optional accelerator to show, and an **availability predicate** over the current target set and
   mode. The module holds no ratatui — the crate boundary that compiler-enforces the core/render
   split is unaffected.

2. **A target is always a set.** Actions receive the selection in tree order, never "the selected
   node". An action meaningful for exactly one row expresses that in its availability predicate
   rather than in its signature, so single-target and multi-target actions are the same shape and a
   later task can widen one without touching its callers.

3. **One `perform(action, targets)`.** Hotkeys, mouse, menu, and socket all route through it. A
   surface that reaches an operation another way is the smell `AGENTS.md` names.

4. **The menu is generated from the action list filtered by availability.** It holds ordering and
   separators and nothing else — no entry list, no per-entry enablement, no knowledge of what any
   action does.

5. **An unavailable action is absent, not disabled.** A greyed row advertises a roadmap rather than
   describing the product, and in a menu whose contents arrive over several sprints most rows would
   be grey on the day it ships. The menu therefore starts small and grows as actions land, and no
   task that adds an action edits the menu.

6. **The right button follows the left.** [ADR 0025](0025-a-click-completes-on-release.md) is
   extended unchanged: a right-click completes on release, on the row its press started on, and
   sliding off abandons it. There is one click model, not one per button.

7. **Right-click targets the selection, and only collapses it when it must.** A right-click inside
   the current selection leaves it alone and acts on all of it; outside it, the selection collapses
   to that row first; below the last row, to the root row. A dimmed row is inert here as everywhere
   ([ADR 0023](0023-narrowing-dims-and-dimmed-is-inert.md)).

# Consequences

- **The menu ships with two entries** — Open and Reveal Root Here — because those are the two
  actions that exist. That is the deliverable: the surface, the layer, and the routing. Each of the
  four queued tasks then adds an action and appears in the menu without touching it.
- **Enablement has one home.** "Can this be done to these rows?" is answered by the action's own
  predicate, for every surface at once. A hotkey and a menu entry cannot disagree.
- **`docs/design.md`'s menu block becomes a specification of the *target*, not of what renders.**
  The rendered menu is whatever is available; the block describes where each entry sits when it
  exists.
- **Segment-scoped menus on compacted chains are dropped.** The menu targets the selection, and a
  chain row contributes its tail like every other surface in birch. `docs/design.md`'s two lines
  promising a segment-scoped menu are corrected to name chain splitting
  ([ADR 0014](0014-chains-split-on-demand.md)) as the way to reach an intermediate directory.
- **A surface exists that a user can open and find nearly empty.** With two entries the menu reads
  as small rather than as broken — but it is a genuine sharp edge until sprint 021 fills it, and it
  is preferable to nine grey rows.
- **The socket does not gain an action verb.** Mutations stay human-initiated
  (`docs/design.md`, Control socket); the layer being shared does not make it addressable.

# Alternatives considered

- **Disabled-and-visible entries.** Rejected: on the day the menu ships, nine of eleven rows would
  be grey, which describes a plan rather than a product. It is also the form that invites a user to
  ask why a thing they can see does not work.
- **Build the menu with its contents — defer `071` until `028` and `029` land.** Rejected when the
  sprint was scoped: the menu is the surface those tasks are blocked on, and each of them is
  materially cheaper to design once the surface and its target model exist.
- **Keep enablement in the menu and let actions stay plain functions.** Rejected: it is exactly the
  menu-specific logic `AGENTS.md` forbids, and it guarantees a hotkey and a menu entry will
  eventually disagree about whether something is permitted.
- **Open the menu on button-down**, as most desktop toolkits do. Rejected: birch has one click
  model, and ADR 0025's reason for it — a click meant only to focus a host pane must be harmless
  and revocable — applies to the button that opens a menu at least as strongly.
