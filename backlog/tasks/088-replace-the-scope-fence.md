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
