---
type: Task
title: Support multi-selection
description: Finder-style selection of several rows with Shift and Ctrl, by mouse and keyboard - requested by the maintainer; currently forbidden by the scope fence, so it needs a fence amendment before design.
status: Draft
priority: medium
---

Maintainer request: select more than one row the way Finder does — Shift-click for a range,
modifier-click to toggle individual rows, Shift with the arrow keys to extend from the keyboard.

## Scope-fence conflict (decide first)

[The design doc](../../docs/design.md) forbids this twice, and the fence is binding:

- **"Permanently out of scope: … multi-select, bulk operations"** (the identity section);
- **"No multi-select"** (file operations).

The fence cannot be amended for the gesture alone: a multi-selection that no action consumes is
decoration, and every action that *does* consume it is a bulk operation — the next item on the same
list. So the amendment has to say how far it goes. A defensible middle stop is **read-only
plurality**: multi-select feeds Copy Paths and `--pick` output, while the mutating ops (delete,
rename) stay strictly single-target. That keeps "bulk operations" out while making the selection
useful. Deciding where the new line sits is a maintainer call and warrants an ADR.

## What a design has to answer

- **Selection becomes a set, everywhere.** Selection is currently a single real path threaded
  through the view model, state persistence (`~/.cache/birch/<root-hash>.json`), the socket, and
  the picker. All four change shape, and persistence keys on real paths, so a restored multi-selection
  must survive rows that no longer exist.
- **The picker contract is a public surface.** `--pick` prints one path on stdout; multi-pick means
  either several lines (and every consumer, including the shipped adapters, must be ready for them)
  or an explicit opt-in flag. Same question for `birch ctl get-path`, which is protocol and evolves
  additively only.
- **Dimmed rows are inert.** [ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md)
  says a dimmed row cannot hold the selection and cannot be acted on. A Shift-click range that
  spans dim rows must therefore *skip* them rather than include them — and the anchor itself can be
  dimmed out from under the selection by the next keystroke.
- **The context menu scopes to the set**, which means entries are enabled or disabled by how many
  rows are selected — the first place the read-only-plurality line becomes visible to a user.
- **Compacted chains.** A chain renders several real directories as one row; whether selecting it
  selects the tail or every segment has no obvious answer.

## Terminal constraints (these bound the design, not the other way round)

- **Cmd is not deliverable.** macOS terminals keep Cmd for themselves, so Finder's ⌘-click has no
  equivalent; the toggle modifier has to be Ctrl (or Alt), and Ctrl-click is a *right*-click on
  macOS by long convention — which collides with `071-add-context-menu`.
- **Shift-click arrives** in the SGR mouse encoding's modifier bits, and **Shift-arrow** arrives as
  a modified sequence (`CSI 1;2A`) in most modern terminals — but not all, and not under every tmux
  configuration. A binding that silently does nothing in someone's terminal is a support burden;
  the design should name the floor it assumes.
- **Shift-drag is reserved** for terminal text selection (the documented mitigation for mouse
  capture), so a Shift-drag *range* selection is not available.

## Related

- `028-add-copy-paths` — the obvious first consumer: several paths, newline-joined.
- `072-drag-to-move` — dragging a whole selection is the natural follow-on ask; the two share a
  fence amendment.
- `071-add-context-menu` — where a multi-selection's available actions become visible.

## Design

[ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md) admits multi-select: the fence is
now the identity, and the explorer panel this identity names has it. The fence question above is
closed, and *"read-only plurality"* is **not** the line drawn — arity is decided per operation when
that operation is designed. What this task builds is the selection itself and the two consumers that
already exist: the picker and the socket.

### The model: one cursor, a set around it

`FlatView::selection` keeps its name and its meaning — **the cursor**: the row `↑`/`↓` move, the row
the socket reports, the row persistence restores, the row a search anchors from. Every existing
reader of it stays correct without change. Two fields join it:

| field | meaning |
|---|---|
| `selection: Option<PathBuf>` | the cursor (unchanged) |
| `extra: HashSet<PathBuf>` | rows selected **besides** the cursor; never contains it |
| `anchor: Option<PathBuf>` | where a `Shift` range starts |

**The cursor is always selected.** The selection is `{cursor} ∪ extra`, and it is empty only when the
cursor is — the case [ADR 0023](../../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md) already
defines, where a narrowing leaves nothing live. There is no "cursor visible, nothing selected" state
to invent, and the single-selection case is byte-for-byte what birch does today with `extra` empty.

`FlatView::selected(&self, rows) -> Vec<&Path>` returns the selection in **tree order** (row order),
which is what every consumer wants. Callers never touch `extra` directly.

**Pruning is `sync`'s job, under one rule: what cannot hold the selection leaves it.** On every
reconcile, a path in `extra` that is not a currently visible **live** row is dropped. Collapsing a
parent, a query dimming a row, and a deleted file therefore all remove it — the same rule, once,
rather than three special cases. This is ADR 0023 applied to a set instead of a single row.

### Gestures

**The keyboard is the guaranteed path.**

| keys | effect |
|---|---|
| `↑` `↓` `→` `←` `PgUp` `PgDn` `Home` `End` | move the cursor, **collapse the selection to it**, re-anchor there |
| `Shift` + `↑` `↓` `PgUp` `PgDn` `Home` `End` | move the cursor and select every **live** row between the anchor and the cursor, inclusive |

`Shift`+`→`/`←` stay unbound: those keys are structural (expand, split, collapse), and a range has
no meaning attached to them.

**The mouse is opportunistic.**

| gesture | effect |
|---|---|
| click | move the cursor, collapse the selection, re-anchor — today's behaviour, unchanged |
| `Ctrl`-click | toggle that row in or out of the selection, and re-anchor to it |
| `Shift`-click | **not bound** — see below |

`Ctrl`-click on the cursor row while other rows are selected drops it and moves the cursor to the
nearest remaining selected row; on the *only* selected row it does nothing, because the cursor is
always selected.

### Why `Shift` + mouse is not taken

The maintainer's request named `Shift`-click for a range. It is not deliverable, for two independent
reasons, and the design declines it rather than shipping a binding that silently does nothing.

1. **birch has already promised `Shift` to the terminal.** `docs/design.md:125` names it the
   mitigation for mouse capture — *"Shift-click passes through to the terminal"* — and
   `birch --help` tells the user so (`crates/birch/src/main.rs:31`). Taking it back removes the only
   way to select text natively in a birch pane.
2. **The flagship terminal does not deliver it.** ghostty's `mouse-shift-capture` defaults to
   `false`, documented as *"the shift key is not sent with the mouse protocol and will extend the
   selection"*. An application may request it with `XTSHIFTESCAPE` (`CSI > 1 s`), which ghostty
   honours unless the user has set `never` — but doing so is exactly reason 1, globally, for the
   whole pane.

So **range-by-mouse does not exist**; `Shift` + arrows is how a range is made. `Ctrl`-click still
builds any discontiguous set by hand, which is the gesture that cannot be expressed from the
keyboard at all.

**A keyboard-only user can build contiguous ranges but not discontiguous sets.** No toggle key is
introduced: printable characters are reserved (ADR 0008), and no such key was asked for. Filed as
`089-keyboard-toggle-for-multi-select` rather than guessed at.

### Ranges, dim rows, and chains

- **A range spans live rows only.** Dim rows inside the span are skipped, never included — a
  `Shift`-extension cannot smuggle an inert row into the selection.
- **A dimmed anchor resets.** If the anchor is no longer live when a `Shift` move arrives, the range
  starts from the cursor instead. One sentence, no stale-anchor states.
- **A compacted chain contributes its tail.** `Row::path` *is* the tail path everywhere in birch
  already (`select_resolved` normalises to it, and `docs/design.md:194` says copy yields the tail's
  path), so selecting a chain row selects one path and no new rule is needed. Segment-level reach
  stays what it is: split the chain with `→` ([ADR 0014](../../docs/adr/0014-chains-split-on-demand.md)),
  then select members individually.

### Under a search

Typing a query dims non-matching rows, and dimmed rows leave the selection by the pruning rule — so
a multi-selection rarely survives a query, which is correct: what is inert cannot be a target.
`Esc` puts it back. `SearchState` gains `saved_extra` and `saved_anchor` beside the existing
`saved_selection` and `saved_scroll`, so backing out of a search restores the whole selection, not
just the cursor.

`Shift` + `↑`/`↓` under a query extend over **matches**, by the same unit rule
`081-standard-navigation-keys` establishes: under a narrowing the unit of movement is a match.

### The picker

`--pick` alone is **unchanged in every respect** — single selection, one path on stdout. The
documented recipe `nvim "$(birch --pick)"` cannot be allowed to start receiving two lines, so
plurality is opt-in:

- **`--multi`** (meaningful only with `--pick`) enables multi-select in the picker and prints one
  path per line on confirm. The flag is the consumer's declaration that it can read more than one.
- **`--print0`** (meaningful only with `--multi`) separates with `NUL` instead of `\n`. Without it
  the output is undecodable for a file name containing a newline, which is legal on every platform
  birch targets; introducing a multi-line contract without the unambiguous form would be shipping a
  known-broken interface.
- Multi-select is **off** in the picker without `--multi`: a selection the mode cannot output is
  decoration.
- In tree mode multi-select is always on — there is no output contract to break.
- `Enter` in the picker confirms the whole selection; `Enter` in tree mode opens **the cursor only**.
  Opening N files means N `--open-cmd` subprocesses, which is a different action and belongs to
  whichever task ever proposes it.

### The socket

`get-path` stays additive ([ADR 0011](../../docs/adr/0011-ndjson-protocol.md)): `data` continues to
carry exactly one path — the cursor — because changing what an existing field means is not an
additive change.

- **New response field `paths: Option<Vec<String>>`**, present whenever there is a selection,
  carrying every selected path in the requested `form`, in tree order. Old clients ignore it; new
  clients read it. A single selection yields a one-element array, so clients need no special case.
- **`birch ctl get-path --all`** prints one path per line. Without the flag the output is exactly
  what it is today.
- **No new verb, and no mutation verb.** `docs/design.md:250` stands.

### Rendering

Every selected row takes the theme's `selection_bg` (and `selection_fg` where the theme swaps it).
The cursor has to stay distinguishable inside the set, and the theme already carries the means:
`SelectionStyle::SoftBarAccent` draws a `▏` accent bar on the selected row, `FullRow` does not.

**The rule: the cursor row draws today's selected look; other selected rows draw the same fill
without the accent bar; and a `FullRow` theme gains the accent bar on the cursor row only while more
than one row is selected.** With a single selection every theme renders exactly as it does now, so
the common case has no visual change at all; the bar appears only when there is something to
disambiguate. No new theme colour is added.

### Persistence: a multi-selection does not survive a restart

`PersistedState` is **unchanged** — no new field, no version bump, no migration, and no question
about restoring paths that have since vanished.

Expansion and scroll are *view* state and restoring them reproduces what the pane looked like. A
multi-selection is *intent* — a working set assembled in the moment to perform one action. Restoring
it hours later silently re-arms an action the user has forgotten choosing the targets for, and the
first destructive menu entry then acts on all of them. The cursor restores as it does today.

### Public-surface delta

| surface | change |
|---|---|
| CLI flag | **`--multi`** — with `--pick`: enable multi-select, print one path per line |
| CLI flag | **`--print0`** — with `--multi`: `NUL`-separate instead of newline |
| CLI flag | **`birch ctl get-path --all`** — print every selected path, one per line |
| socket | **`Response.paths: Option<Vec<String>>`** — additive, optional, tree-ordered, in the requested `form` |
| key bindings | `Shift` + `↑` `↓` `PgUp` `PgDn` `Home` `End`; `Ctrl`-click |
| config keys | none |
| environment variables | none |
| on-disk paths / formats | none — persistence is untouched |
| library API | none (`birch-core` gains only the `paths` field on an existing type) |

### Implementation

- **`crates/birch-tui/src/flat_view.rs`** — `extra`, `anchor`, `selected()`, range extension, toggle,
  and the prune in `sync`. Every existing mutator (`select`, `select_resolved`, `move_by`,
  `on_right`, `on_left`, `on_single_click`, `on_click`) collapses the selection unless it is the new
  `Shift`/`Ctrl` path, so "a plain move clears the set" is enforced in one place rather than
  remembered at each call site.
- **`crates/birch-tui/src/input.rs`** — `Shift`-modified variants of the six movement keys, and the
  modifier bits on `Press`/`Release` (`mouse.modifiers` is currently discarded).
- **`crates/birch-tui/src/render.rs`** — the accent-bar rule; `hit_test` is unaffected (geometry does
  not change).
- **`crates/birch/src/app.rs`** — dispatch for the new actions, `SearchState::saved_extra`/
  `saved_anchor`, the picker's multi confirm, and `Verb::GetPath` filling `paths`.
- **`crates/birch-core/src/protocol.rs`** — the `paths` field.
- **`crates/birch/src/main.rs`** / **`ctl_client.rs`** — `--multi`, `--print0`, `--all`.

### Tests and verification

Unit: range extension over a fixture containing dim rows (they are skipped); a dimmed anchor
resetting; `Ctrl`-toggle on the sole selection being a no-op; the prune dropping a collapsed
subtree's members; `selected()` returning tree order; `Esc` restoring the pre-search set; `paths`
present and tree-ordered for one and for many; `--pick` without `--multi` still printing exactly one
line. Hands-on: `vhs` frames for a multi-row selection under both `SelectionStyle`s (a visual change
— required), and driven-binary checks of `--pick --multi`, `--print0`, and `ctl get-path --all`.

### Open measurement

Whether `Ctrl`-click reaches a TUI as a left button carrying `CTRL` — rather than being translated
to a right-click, which is the macOS convention — is terminal behaviour, not birch's to decide.
`.scratchpad/mouse-probe.py` enables exactly the modes birch enables and reports what arrives. If
`Ctrl`-click does not survive, the fallback is `Alt`-click, and the keyboard path is unaffected
either way.
