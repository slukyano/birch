---
type: Sprint
title: Multi-select and the action surface
status: Done
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

- [x] 081-standard-navigation-keys — implemented, gates green, verified in a pty
- [x] 088-replace-the-scope-fence — implemented (documents only)
- [x] 074-add-multi-selection — implemented, gates green, verified in a pty and with vhs
- [x] 071-add-context-menu — implemented, gates green, verified in a pty and with vhs

# Open questions

_(none open)_

# Sprint summary

Branch `sprint/020`, base `main`. Six implementation commits on top of the approved design merge.
278 tests pass; `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` clean.

# Task ledger

Weight is size **and** importance **and** future impact together, so a small diff can be major —
`088` changes four files and governs every scope question birch will ever face.

| task | weight | what it is |
|---|---|---|
| `088-replace-the-scope-fence` | **major** | The permanent out-of-scope list is deleted from `docs/design.md`; the identity sentence becomes the whole fence, judged in two halves (the explorer panel's job, and a narrow terminal pane as the medium), the in-scope line becomes a description that gates nothing, and `AGENTS.md`'s reject-by-lookup rule becomes that judgement test. Delivered as designed. |
| `081-standard-navigation-keys` | **mid** | `PgUp`/`PgDn`/`Home`/`End` were dead keys; they now move the selection and let the viewport follow, by a page of live rows in a free tree and a page of matches under a query, clamping where a single step wraps. Delivered as designed. |
| `074-add-multi-selection` | **major** | Selection becomes a set around the cursor: `Shift` with the movement keys extends a range, `Ctrl`-click toggles, `Shift`-click extends where the terminal forwards it. `--pick --multi` and `--print0` for the picker, an additive `paths` array on `get-path`, and `birch ctl get-path --all`. Delivered as designed. Two decisions inside that design are worth naming: `Shift`-click was designed out on evidence and restored by maintainer decision **before** the design merge, and a multi-selection deliberately does not persist — which removed the sprint's own "restored selection must survive vanished rows" risk. |
| `071-add-context-menu` | **major** | Right-click opens a menu generated from a new action layer in `birch-core`, rather than an authored list. Ships with the two actions that exist. Delivered as designed. |

All four were delivered; none was dropped or reduced. One promise was narrowed during the design
phase — segment-scoped context menus on compacted chains, withdrawn so that every surface resolves a
compacted chain to its tail, with `docs/design.md` corrected to name chain splitting as the reach.

# Breaking changes

**None** — no output, wire format, or on-disk format changed. **Two keyboard behaviours did change
meaning**, which is the feature rather than a break, but a user with muscle memory will notice:

- **`Shift` with `↑`/`↓` used to be a plain move** — the modifier was ignored — and now extends a
  selection. The cursor still lands in exactly the same place, so nothing that reads the selection
  decodes differently; what changes is that rows behind it stay selected.
- **`Ctrl`-click used to be a plain select** and now toggles a row in or out of the selection.

Everything else was held deliberately:

- `birch --pick` without `--multi` is byte-for-byte what it was: single selection, exactly one line on stdout. `nvim "$(birch --pick)"` and every shipped adapter keep working.
- `get-path` still carries exactly one path in `data` — the cursor. The selection arrives in a new optional `paths` field, which a client that has never heard of it ignores (ADR 0011, additive-only).
- `PersistedState` is untouched: no new field, no version bump, no migration.
- `↑`/`↓` under a search wrap exactly as before, pinned by test through the `step_match` refactor.
- Every theme renders a single selection exactly as it did.

# Backlog changes

Two gaps surfaced while designing and were filed rather than guessed at. Informational — neither is
part of this sprint's scope.

| task | why it exists |
|---|---|
| `089-keyboard-toggle-for-multi-select` (`Draft`) | `Shift` with the movement keys extends a contiguous range, and `Ctrl`-click toggles a row, but there is no keyboard equivalent of the toggle — so a *discontiguous* selection is reachable by mouse only. No toggle key was asked for, and printable characters are reserved, so the binding decision was left to its own task. |
| `090-hover-highlight-in-the-tree` (`Draft`) | The context menu enables motion tracking for its own lifetime, because a menu that does not light the entry under the pointer cannot be used with a mouse. Hover in the tree is a different feature with a different event-volume cost. |

# Architectural decisions

- **ADR 0028** — the identity sentence is the whole scope fence, tested in two halves that must both hold. The permanent out-of-scope list is deleted with nothing replacing it; prohibitions that carry their own reasoning stay in the sections that argue them. Multi-select is admitted; drag-to-move becomes arguable rather than settled.
- **ADR 0029** — the context menu is generated from an action layer, not authored. Actions declare a label, an accelerator, and an availability rule over a target set; one `perform` serves every surface; an unavailable action is absent, not greyed; the right button follows the left's click model.

# Bugs found and fixed

**Three from reviewing the sprint diff** (`a0087f8`):

1. **Motion tracking leaked past exit.** SGR 1003 is enabled while the menu is open, and crossterm's `DisableMouseCapture` does not cover that mode — so `Ctrl-C` with the menu open, or a SIGHUP from a closing pane, left the terminal reporting every pointer move afterwards. `term::restore` now clears it unconditionally.
2. **A right-click elsewhere stranded the menu** on the row it opened on. It now closes the old menu and arms the press, so the release opens a new one where the click landed.
3. **A doc comment claimed "nearest"** where toggling the cursor out moves it to the first row still selected in tree order.

**Seven from the independent review** — a fresh reviewer with no implementation context, reading the whole diff and running the gates itself (`8cbc2af`):

4. **A pick made from the context menu never ended the picker.** The menu branch returned before the terminator every other input path falls through to, so the pick sat until whatever key arrived next — and an `Esc` or a printable in between went to the search instead. The terminator is now named and shared.
5. **Clicking a menu separator invoked the standing highlight.** `highlight` ignores separators by design, so moving it and reading it back fired the previous entry. The clicked row is now read directly.
6. **Backspacing out of a search left a range extension armed**, and the range was repainted every frame — so rows that were dim when the range was made joined it the moment the narrowing lifted, and a watcher delta inserting a row inside the span did the same. Under `--pick --multi` that printed a path the user never selected. A range is now painted once, when the cursor lands.
7. **A plain move that went nowhere did not collapse the selection** — at an edge, or `→` with nothing left to reveal — leaving the extension armed and finding 6's mechanism live.
8. **The right button armed and released on an exact cell**, so a pointer drifting one column between press and release silently opened nothing. It resolves to a row like the left button.
9. **The menu re-targeted the root for any `hit_test` miss.** That function says `None` for the space below the last row, the scrollbar column, and the status line alike; right-clicking the scrollbar destroyed a multi-selection and jumped the cursor to the root. `render::on_tree_surface` tells them apart.
10. **A right-click on a dimmed or git-deleted row collapsed the selection** on the way to discovering there was no menu to open.

**And one more the review could not have seen**, found while verifying fix 6 in a real terminal: a `Shift` extension **resolved its anchor after the cursor had moved**. Under a search the anchor is routinely stale — `sync` re-homes a cursor without moving the anchor, and a query dims the row it sits on — so the range collapsed to a single row whenever the search index arrived after the query was typed. Every unit test passed throughout, because in a test the index is already built. The anchor is now resolved before the move, falling back to the cursor.

# Verification

Gates: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, 278 tests. Every new
branch carries a test; the reviewer's list of uncovered branches was closed by making menu input,
action performance, and menu opening terminal-free, so each is reachable without a terminal.

Hands-on, all run and observed:

- **`081`** — driven in a 24×80 pty with the selection read back over the control socket after each key: `PgDn` steps 22 rows, a second `PgDn` another 22, `PgUp` returns, `End` reaches the last row, `PgDn` past the end holds there, `Home` reaches the root row. Under a search for `f5` (11 matches), `End` and `Home` reach the last and first match and paging clamps on both.
- **`074`** — the same instrument: `Shift-Down` extends with `data` reporting the cursor and `paths` the whole set, `Shift-Up` shrinks it, a plain `Down` collapses it, and backspacing a query away leaves the range exactly as it was rather than swallowing the row that had been dim inside it. `--pick` alone printed exactly one line with `Shift` ignored; `--pick --multi` printed three; `--print0` NUL-separated them. `vhs` frames in both `SelectionStyle`s: a single selection renders identically to before, and in a range the wash spans every selected row with the accent bar marking the cursor.
- **`071`** — `vhs` frames of the menu on a row, and of a right-click below the last row targeting the root with the menu shifted flush inside the pane's right edge. Driven checks: choosing **Open** in the picker exits immediately printing the path; `Esc` closes the menu without quitting and only the second `Esc` quits; a printable closes it and reaches the search; `1003` is requested on open and released on close.

Nothing was skipped, and no scenario needed an instrument that was unavailable.

# Remaining limitations and highlights

- **The menu ships with two entries.** Open and Reveal Root Here are the only actions that exist. This is the approved split — the surface first, its contents in sprint 021 — but a user who opens the menu this sprint will find it nearly empty.
- **Which mouse modifiers arrive is the terminal's decision, not birch's.** ghostty's `mouse-shift-capture` defaults to `false`, so `Shift`-click is kept for its own text selection and never reaches birch there; `Ctrl`-click may be translated to a right-click by macOS convention. birch handles either wherever it is forwarded and deliberately does not send `XTSHIFTESCAPE` to claim `Shift` back, which would withdraw the native text selection for the whole pane. Nothing in the behaviour depends on the answer, and `Shift` with the arrow keys always works.
- **A keyboard-only user can build contiguous ranges but not discontiguous sets.** `089-keyboard-toggle-for-multi-select` holds this.
- **A narrowing evaporates a multi-selection.** Typing a query dims non-matching rows, and a dimmed row is inert (ADR 0023), so it leaves the selection. `Esc` restores the whole set, not just the cursor.
- **A multi-selection does not survive a restart**, by design: expansion and scroll are view state, but a selection is intent, and restoring it hours later would re-arm an action whose targets have been forgotten.
- **`--multi` output is newline-separated unless `--print0` is given**, and a newline is a legal character in a file name. `--print0` exists precisely for that and is the safe form for scripts.

# Session log

- Scoped and cut: `081`, `088`, `074`, `071`. Branch `sprint/020` cut from `main`.
- Design: `088` and `081` presented first, since `088` gates `074`; four questions answered
  (the two-part identity test, per-operation arity, selection-moving page keys, and paging by
  matches under a query). `074` and `071` designed on the settled fence.
- `Shift` with the mouse was first designed away and then **restored by maintainer decision**:
  birch handles the gesture whenever the terminal forwards it, and does not send `XTSHIFTESCAPE` to
  claim it. ghostty's `mouse-shift-capture` defaults to `false`, so the event does not arrive there;
  that is the terminal's call, and the keyboard path is the one that always works.
- Two gaps found while designing were filed rather than guessed at:
  `089-keyboard-toggle-for-multi-select` and `090-hover-highlight-in-the-tree`.
- ADRs proposed: 0028 (the identity is the scope fence), 0029 (the menu projects the action layer).
- Close-out: gates green (fmt, clippy, 278 tests, hands-on verification of all four tasks,
  publication hygiene). An independent reviewer with no implementation context read the whole diff
  and found seven defects, all fixed on the branch; verifying one of those fixes in a real terminal
  surfaced an eighth — a `Shift` extension resolving its anchor after the cursor had moved, which no
  unit test could see because the failure depends on the search index arriving after the query.
