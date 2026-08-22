# birch

Lean and beautiful interactive file tree for the terminal.

## Identity

birch does exactly what an IDE file tree does, and nothing else.

It is the IDEA / VS Code explorer panel, living in a terminal pane next to your editor. That sentence is the whole scope fence, and it has two halves that must both hold ([ADR 0028](adr/0028-the-identity-is-the-scope-fence.md)):

- **The panel's job.** Does the IDEA / VS Code explorer panel do this? The panel is a tree of files plus the operations reached from a row — not a file manager, not a diff viewer, not a shell, not a second navigation model.
- **The pane's medium.** Does it survive a narrow terminal pane where every printable character belongs to search ([ADR 0008](adr/0008-q-types-into-search.md))? A feature needing width, a second axis, a modal state, or a letter hotkey fails here however ordinary it is inside an IDE.

**What that is today:** tree view, icons, git status, open file, fuzzy find, content search, multi-select, rename / delete / new file / new dir, copy name/path, compact folders, live updates, mouse. That line describes the current surface; it gates nothing. A feature absent from it is not thereby out of scope — the identity decides. Prohibitions stated by a section below (no plugins, no search-and-replace, no mutation verbs on the socket, no copy, no move) stand on their own terms.

Contrast with neighbors:

- Not **yazi** — a tree, not miller columns.
- Not **broot** — no verbs, no modal behavior. Dead simple: arrows navigate, typing searches, Enter opens.
- Not **eza** — interactive and live, not a one-shot print.

birch is designed to run persistently in a side pane (herdr, cmux, tmux) — the pane integration story is a first-class concern, not an afterthought.

## Invocation

```
birch [<options>] [<dir>]
```

`<dir>` defaults to cwd and becomes the tree root. The root is fixed for the life of the instance except via `Reveal Root Here` (context menu) or `birch ctl set-root`.

```
birch --pick [<dir>]      # picker mode: Enter prints the selection and exits
birch --pick --multi      # ...several: one path per line (--print0 to NUL-separate)
birch --filter '*.md'     # glob filter (repeatable); --filter-mode hide|skip
birch ctl <verb> [...]    # control a running instance (see Control socket)
```

## Appearance

- **The root is the first row**: the directory name plus its full path as a dim,
  `~`-abbreviated annotation (IDEA-style); the bottom line stays clear for messages. The
  root behaves as a dir row — collapsible, clickable — but never joins a compact chain
  and is exempt from visibility filters (a dot-dir root still shows). In `--pick`,
  Enter on the root row picks the root itself.
- **Nerd Font icons** per file type. Best with terminals shipping a Nerd Font fallback (ghostty).
- **Git status** badges + colors, VS Code-style: modified, added, untracked, deleted, renamed; ignored files dimmed. Status **propagates to ancestor dirs**, so a collapsed tree still shows where the changes are.
- **Deleted-but-tracked files are shown** in deleted state — the tree reflects git state, not just the filesystem.
- **Compact folders** (VS Code-style): pure single-child dir chains render as one row, `a/b/c`, separators dimmed. Rules:
  - A chain compacts iff each dir's only *visible* child is a single dir (visibility-aware: hidden-file settings affect tree shape).
  - Compaction is a **render-layer transform**. The real tree, watcher, git, search, and ops all speak real paths; compaction happens at paint time only.
  - Keyboard treats a chain as one node (the tail) — until `→` on an expanded chain **splits** it into individual member rows; collapsing a member re-fuses the chain from that point (ADR 0014). Splits are session-local. Every surface resolves a chain row to its **tail** — selection, `get-path`, the copy variants, and the context menu alike; reaching an intermediate directory means splitting the chain first (ADR 0029).
  - F2 on a chain inline-edits the full `a/b/c` fragment (this is rename-with-path reused).
  - Live updates split/fuse chains in place as children appear/disappear.

## Defaults

| Setting | Default | Initial flag |
|---|---|---|
| Icons | on | `--no-icons` |
| Git status | on | `--no-git` |
| Gitignored files | shown, dimmed, **auto-collapsed** | `--hide-ignored` |
| Hidden (dot) files | shown | `--hide-hidden` |
| Noise (`.git`, `.DS_Store`, …) | hidden | `--show-noise` |
| Compact folders | on | `--no-compact` |
| Mouse | on | `--no-mouse` |
| Theme | `birch` | `--theme <id>` |
| Scroll lines per wheel tick | 3 (range 1–10) | `--scroll-lines <n>` |
| Scrollbar | on (shown only when rows overflow) | `--no-scrollbar` |
| Badge placement | the theme's choice | `--badges <right\|left\|none\|auto>` |
| Branch badges (vs the remote base) | on where a base resolves | `--no-branch-badges` |

Directories always sort before files. Defaults persist in the config file
(`~/.config/birch/birch.toml`, ADR 0022); boolean flags are bidirectional, and the precedence is
config < flags < `birch ctl set`. The theme catalog is ADR 0021.

Flags set initial values; runtime changes go through `birch ctl set` (and possibly a few hotkeys later — added sparingly).

Ignored dirs (e.g. `node_modules`) are never auto-expanded, never searched, and never recursively watched.

## Interaction

### Keyboard

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection (during a search: step matches, in tree order) |
| `PgUp` / `PgDn` | Move selection a page — the viewport less a row of overlap — and let the viewport follow (during a search: a page's worth of matches). Clamps at the ends where `↑`/`↓` wrap. |
| `Home` / `End` | Move selection to the first / last row (during a search: the first / last match) |
| `Shift` + `↑` `↓` `PgUp` `PgDn` `Home` `End` | Extend the selection from the anchor to where the cursor lands, over live rows only. Any unmodified move collapses the selection back to one row. |
| `→` | **Always moves or reveals** (task 060): a collapsed dir expands; an expanded chain splits into member rows; otherwise the selection advances to the next row. Nothing happens only on the last row. |
| `Enter` on dir | Toggle expand/collapse |
| `←` on dir / into parent | Collapse / jump to parent |
| `Enter` on file | Open (see Opening files). **Enter always opens — never contextual.** |
| any printable char | Fuzzy filename search |
| `Shift-F10` / `Menu` | Open the context menu at the cursor row, without disturbing the selection. On an Apple keyboard the `Menu` key does not exist and `F10` is Mute, so the gesture is `Fn`-`Shift`-`F10` unless standard function keys are enabled. |
| `Esc` | Back out one layer: cancel an inline edit or a confirmation, else close the context menu, else clear the search; at top level, quit (ADR 0012, ADR 0030) |
| `F2` | Rename (inline) |
| `Delete` | Delete (to trash) |
| `Ctrl-N` | New file / dir |
| `Ctrl-F` | Content search source |
| `Ctrl-C` | Quit (`q` is a search character like any other) |

Printable characters are permanently reserved for search — no letter hotkeys, ever. The context menu is the primary surface for everything else; hotkeys above are accelerators shown in the menu.

### Mouse (native, on by default)

- **Single-click selects; double-click activates** (ADR 0015 — reversed from the original VS Code-school single-click-activates). In a host pane, a click meant to focus the pane — or just to point — arrives as a normal click, so the first click must be harmless: it only moves selection. A second click on the same row within 450 ms is Enter's twin (open file / toggle dir). Clicking a dir's chevron toggles without moving selection; each chevron click is its own toggle.
- **A click completes on release** (ADR 0025): the press arms, and the click acts only when the button is released on the same row, in the same zone. Releasing anywhere else abandons it, so a click can be revoked by sliding off. The double-click window measures release to release — two complete clicks.
- **Drag a row onto a directory to move it** ([ADR 0032](adr/0032-move-by-dragging.md)). A drag is a click that left its row — no threshold, since the click model already tracks which row a press began on — so a click that wobbles inside its row is still a click. A press inside the selection drags all of it; outside it drags that row alone. A file targets its parent, below the last row targets the root, and a compacted chain targets its tail. Hovering a collapsed directory springs it open, except an ignored one, which stays shut while remaining a legal target. **Refusals are visible before release**: a folder into its own descendant, a colliding name, or the source's own parent draw no highlight and the status line says why. A drop never overwrites and never silently renames, and the status line reports what moved and where. **`Ctrl-Z` takes a drop back** ([ADR 0033](adr/0033-undo-is-a-session-stack.md)), including a multi-row one, as a single step. Dragging into the pane's edge scrolls a row per movement; holding still does not keep scrolling.
- **`Ctrl`-click toggles** a row in or out of the selection; **`Shift`-click** extends the range from the anchor to it. Both are handled whenever the terminal forwards the modifier — many terminals keep `Shift` for their own text selection while an application has the mouse (ghostty's `mouse-shift-capture` defaults to `false`), and birch never claims it back. `Shift` with the arrow keys always works.
- **Scroll wheel**: `--scroll-lines` rows per tick, 3 by default (1–10).
- **Hover highlight** (SGR 1003 motion tracking): motion is reported for the whole session — `EnableMouseCapture` requests mode 1003 along with the rest, and birch never turns a mouse mode off while it runs, because resetting one drops reporting below the level the terminal was told to use. The context menu is the only thing that acts on motion today; hover in the tree is `090-hover-highlight-in-the-tree`.
- **Right-click → context menu**, which is a **projection of the action layer** (ADR 0029), not an authored list: actions declare a label, an accelerator, and an availability rule over the selection, and the menu draws whichever are available. **An action that does not exist yet is absent, not greyed** — the menu grows as its contents land, and no task that adds an action edits the menu. Its full intended shape, in groups (a separator is drawn between any two groups that survive):

```
Open with nvim        ↵      (a file: the resolved open command)
Expand / Collapse     ↵      (a directory: what Enter will do to it)
Open with open               (the platform opener, named: open / xdg-open)
Reveal                       (show it in the desktop's file manager)
──────────
Undo Rename of b.txt  ^Z     (093 — names the step it would reverse)
──────────
New File…             ^N     (029)
New Folder…                  (029)
Rename…               F2     (029)
Delete                ⌦      (029)
──────────
Copy Name                    (028)
Copy Relative Path           (028)
Copy Absolute Path           (028)
──────────
Reveal Root Here
```

- **Every row describes what will happen to *this* selection**, not the category it belongs to. The `Open` row names the program it will run — the program's name, not the whole template, which truncated is noise — and the name is drawn **dim**, so the row reads as an instruction and its parameter rather than as one compound verb. `Open` acts on the **cursor**, not the selection, so its label follows the cursor's row and nothing else: on a directory `Enter` toggles, and the row says `Expand` or `Collapse` accordingly, because naming an editor that will not run describes the wrong thing. In `--pick` it confirms, and the row is a plain `Open`. The second row hands the path to the desktop's own handler, **named** rather than described — `Open with open`, `Open with xdg-open` — because "system" is not a thing anybody runs. It is absent when the resolved command already *is* that handler, so the menu never shows two rows that do one thing, and absent in `--pick`.
- **A folder gets no plain `Open` row**, which is where birch's menu and VS Code's part ways for a reason: VS Code drops its open entries on a folder entirely (`ExplorerFolderContext.toNegated()`), because there the toggle is only ever a click. birch's menu also teaches the keyboard — it prints the accelerator beside each row — so naming what `Enter` does is worth a row where hiding it is not.
- **It opens from the keyboard too**, with `Shift-F10` or the `Menu` key, at the cursor row. A right-click has to reconcile a pointer with the selection; a keyboard trigger has no such conflict and moves nothing.
- **Opening it follows the left button's rule**: a right-press arms, and the menu opens on the release on the row the press started on (ADR 0025, extended). Keyboard works inside it — `↑`/`↓` step entries and skip separators, `Home`/`End` reach the ends, `Enter` invokes, `Esc` closes. **A printable character closes the menu and types into the search**: ADR 0008 has no exception, and a layer that redefines the keyboard is how a modal language starts.
- **It targets the selection, and only disturbs it when it must**: a right-click inside the current selection acts on all of it, a click outside collapses the selection to that row first, and a click below the last row collapses to the root. A dimmed row is inert here as everywhere (ADR 0023).
- **Geometry**: drawn as a bordered box over the tree, shifted left to fit at the right edge (rather than flipped, which in a narrow pane throws it across the whole width), flipped upward at the bottom, and clamped-and-scrolled in a pane too short to hold it. Accelerators are dropped before labels ellipsise.
- **Text-selection trade-off**: mouse capture disables native drag-to-copy. Mitigations: most terminals pass `Shift`-drag through to their own selection while an application holds the mouse (documented in `--help`) — it is the terminal's decision, not birch's; Copy Path in the menu covers the real use case.
- No drag-and-drop. Inside tmux, mouse requires tmux mouse mode (FAQ, not a bug).

### Opening files

`Enter` / double-click runs the open command. Default: `$VISUAL {}` (else `$EDITOR {}`) if set, else `open` (macOS) / `xdg-open` (Linux).

```
--open-cmd '<template>'     # {} = path (appended when absent)
--open-detached             # the template is fire-and-forget: spawned with
                            # null stdio, no terminal handover (adapters)
```

A custom template hands the terminal over and waits (terminal editors) unless `--open-detached` marks it detached from the tty — the mode adapter open-cmds need, since they are short scripts driving the host's CLI, not tty programs.

Examples: `nvim {}`, `code -r {}`, or a host adapter's open primitive (herdr types into the main pane; cmux opens previews as a tab in the current workspace). The template grows a `{line}` placeholder when content search lands (the feature that produces line numbers); until then the contract is `{}`-only.

## Search

### Filename fuzzy search (just type)

- **Jump, not filter**: non-matches dim, matches highlight; `↑`/`↓` cycles matches **in tree order**, so the selection travels down the pane rather than by relevance; tree stays spatially stable (it's an ambient pane you glance at constantly). `Esc` restores.
- **A dim row is inert** (ADR 0023): it stays on screen for context, but the selection cannot land on it — arrows step over it, a click does nothing, `Enter` never acts on it. With no matches there is no selection at all. The selection anchors forward on every keystroke: the first match at or after the current row, wrapping at the end.
- Whole-tree scope, auto-expanding to reveal matches, **never descending into ignored dirs**.
- Matches **simple names**; a query containing `/` matches full root-relative paths
  instead. Matched characters highlight in place — including inside compacted labels
  (`a/b/c`). Uppercase query characters anchor to capitals (camel humps).
- `--pick` behaves identically (ADR 0023). One engine, one render policy: the picker was the mode that most needed the surrounding context, and two models behind one keystroke taught two mental models.

### Content search (Ctrl-F) — a source, not a mode

Content search swaps the pane's **source** (see Architecture): same tree widget, same keys, different nodes — files-with-matches, expandable to match lines:

```
 src
   parser.rs          4
     ▸ 112: fn parse_tree(
     ▸ 208: // parse_tree is called…
```

- Typing edits the query (no conflict — filename fuzzy is meaningless here). ~150 ms debounce, in-flight searches cancelled on keystroke, results live.
- `Enter` on a match line opens at the matched line — this phase adds `{line}` to the
  `--open-cmd` template contract (args containing it are dropped when no line applies,
  so `nvim +{line} {}` degrades to `nvim <path>` in the Files source).
- `Esc` returns to the Files source, cursor/scroll restored.
- Built on ripgrep's crates (`grep-searcher`, `grep-regex`, `ignore`) — no subprocess, no PATH dependency, incremental + cancellable. The `ignore` crate also provides the tree's gitignore logic: one dependency, both jobs.
- Smart-case on; no regex/case/word toggles in the UI (available via `birch ctl set`).
- Respects the ignore boundary. Live updates re-search touched files only.
- **No search-and-replace, ever** (mutation-across-files; different product).

## File operations

**Rename, delete, new file, new dir — and move, by dragging** ([ADR 0032](adr/0032-move-by-dragging.md)). No copy. **Undo is a session stack** ([ADR 0033](adr/0033-undo-is-a-session-stack.md)): `Ctrl-Z` reverses the last mutation, the menu row names what it would reverse, and every step is validated against the filesystem when it runs rather than when it was recorded. **Delete is taken back too**: birch keeps what each trashed path needs to come back — where the OS put it, or the platform's own trash record — and `Ctrl-Z` restores it, refusing rather than overwriting if the old name has been taken meanwhile. No redo. Whether an operation accepts one target or several is settled per operation: **delete takes the whole selection**, while rename and create take exactly one — a rename edits one row, and several rows do not name one directory to create in. All four are unavailable in `--pick`, where mutations are disabled.

**Typing while an edit is open reaches the field, not the search.** [ADR 0030](adr/0030-the-inline-editor-is-the-only-mode.md) reads [ADR 0008](adr/0008-q-types-into-search.md) as it was always meant — printables go to whatever text field is focused, and an edit focuses a different one — so no letter becomes a command. The inline editor is birch's **only** mode, admissible because it is self-evident, transient, and singly-exited; a second one has to clear the same bar.

- **Rename (F2)**: inline row editing — the selected row becomes a text input (never a bottom command bar; prompt bars grow verbs). Editing to a path (`../other/name.ts`) silently performs a move — rename-with-path is the 80% of "move" without the complexity cliff. Plain `rename()`; no `git mv` magic (git rename detection handles it).
- **New file (Ctrl-N)**: inline input in the target dir. `foo/bar/baz.ts` creates intermediate dirs; trailing slash (`foo/`) creates a dir. One binding, three ops (VS Code's trick). On a compacted chain, creates in the tail. No dedicated new-dir key.
- **Delete (⌦)**: **to trash** (macOS trash / XDG trash spec) — undoable beats confirmable. There are two safety nets, and the friction goes where neither reaches ([ADR 0031](adr/0031-delete-goes-to-trash.md)): a selection whose every path is *tracked* in git deletes immediately, since it is recoverable twice over; anything *untracked* confirms once, because it exists nowhere else and a trash can be emptied; and when the trash is unavailable **nothing is deleted** — the failure is reported and the delete ends there. **There is no permanent delete**, neither as a fallback nor as a choice: this whole design rests on delete being recoverable, and a route around the trash removes the foundation. **Confirmations answer to `Enter` and `Esc`, never `y`/`n`** — a letter answering a prompt is the letter command ADR 0008 forbids. On an Apple keyboard the key labelled *Delete* sends Backspace, which belongs to the search, so the gesture there is `Fn`-`Delete`.
- Ops and the watcher share the tree-delta stream: the op layer suppresses/owns watcher events for paths it is mutating (no flicker, no selection jumps mid-edit).
- Mutations are **human-initiated only** — no socket verbs, and disabled by default in `--pick` mode.

## Copy name / paths

Three variants, on the context menu. **No accelerator**: `Ctrl-Shift-C` cannot be bound, because in the key encoding birch decodes with `Shift` has no representation alongside a control character — it is the same byte as `Ctrl-C`, which quits. Reaching for copy would quit birch. Making it possible needs the Kitty keyboard protocol (`091-kitty-keyboard-protocol`).

- **Copy Name** — `NewThing.tsx`
- **Copy Relative Path** — relative to the **birch root** (not cwd; root-relative is what you're looking at)
- **Copy Absolute Path**

Raw paths, no quoting/escaping magic (guessing the paste context is a losing game). On a compacted chain: tail's path; segment-click reaches intermediates.

With several rows selected, a copy joins them with newlines, in tree order.

**Clipboard mechanics.** **One transport per copy, chosen from the environment** — never two written together, so a clipboard manager sees one event and nothing fights another process for an X11 selection. OSC 52 is a one-way write with no reply, so a *reactive* chain cannot be built: there is no failure to observe and react to.

1. **A remote session** (`$SSH_TTY`) takes **OSC 52** — the only transport that reaches the terminal the user is sitting at. A native tool there writes a clipboard nobody will ever paste from.
2. **Otherwise the first native tool available** — `pbcopy`, else `wl-copy` under Wayland, else `xclip`/`xsel` under X11. This is not a backstop: Apple Terminal does not support OSC 52 at all.
3. **With no native tool**, OSC 52 — whose silence costs nothing there, since there is nothing to fall back to.

The status line then confirms what was copied. It is not a fallback tier: nothing here can know that a copy failed.

"Resolve selected node → name/relpath/abspath" is one primitive with three transports: clipboard (menu), stdout (`--pick`), socket (`get-path`).

## Live updates

The tree watches the filesystem and git state; updates are immediate:

- Create / delete / rename → tree updates in place; compacted chains split/fuse.
- Git status changes → badges and ancestor propagation update.
- Selection stays stable when rows appear/disappear above it.
- **Watch only expanded dirs** — recursive watching a 500k-file monorepo eats inotify limits. This constraint shapes the architecture; it is not an optimization to add later.

## Architecture

Two boundaries are load-bearing; everything else is negotiable.

1. **Sources are delta streams.** A source emits tree deltas (add / remove / update node); the view renders deltas. Files is the default source; Content Search is the second (and validates the interface); Git Changes and Project View come later for free. If the core is "readdir + decorations," sources require a rewrite.
2. **Real tree vs. render layer.** All logic — watcher, git, search, ops, socket — speaks real paths. Compaction, dimming, badges, search highlighting are paint-time transforms. Persisted state keys on real paths (so visibility toggles that change tree shape don't corrupt restored state).

Other notes:

- **Stack**: Rust + ratatui + notify + gix + ripgrep crates. Perf headroom for 100k-file trees; mature watcher/git/search ecosystem.
- **Workspace layout** (internal crates; `publish = false` for now):
  - `birch-core` — real tree, sources-as-delta-streams, watcher, git status, search, ops. **Must build without ratatui** — the crate boundary compiler-enforces the real-tree/render split.
  - `birch-tui` — render layer: compaction, badges, widget, mouse, context menu, inline edit.
  - `birch` — binary: wiring, flags, socket server, and the `birch ctl` control client.
  - Publishing these as a library ("libbirch") is deliberately deferred: no named external consumer exists, and SemVer stability would tax exactly the interfaces (sources, deltas) most likely to change. Revisit if a concrete embedder appears — the live candidate is herdr wanting the tree in-process rather than as a pane. Extraction from well-factored crates is cheap; un-publishing an API is not.
- **State persistence** (nice-to-have, cheap): expansion, selection, scroll per root at `~/.cache/birch/<root-hash>.json`; restored on launch. Crash/reboot resilience.
- Actions (open, rename, copy-path, …) live in one action layer consumed by hotkeys, mouse, context menu, and socket alike. Menu-specific logic is a smell.

## Control socket

A running birch instance exposes a Unix domain socket; `birch ctl` (a subcommand of the birch binary) is the thin client.

- **Addressing**: socket per instance at `$XDG_RUNTIME_DIR/birch/<pid>.sock`; per-root symlink `by-root/<root-hash>.sock` → most recent instance. `birch ctl` resolves by walking up from cwd to a matching root — `birch ctl get-path` in a sibling pane just works.
- **Host-dictated rendezvous**: `birch --socket <path>` binds exactly there, skipping default addressing (`--no-socket` skips binding entirely). A host that spawns birch shouldn't have to *discover* what it created — it picks the path and already knows it. `birch ctl --socket <path>` targets it. This is the load-bearing flag for host adapters.
- **Lifecycle**: graceful exit on SIGHUP/SIGTERM (host closes pane → clean death, socket unlinked).
- **Protocol**: newline-delimited JSON request/response with a `v` field. **Additive-only evolution; clients tolerate unknown fields.** Host adapters may be maintained out-of-tree on their own release cadence — the protocol is a public API commitment, not an internal detail.
- **Verbs** (a closed set, for its own reason: the protocol is a public API and stays small):

| Verb | Effect |
|---|---|
| `reveal <path>` | Expand to and select path (editor → tree reverse integration) |
| `get-path [--name\|--rel\|--abs] [--all]` | Print current selection; `--all` prints every selected path, one per line. On the wire `data` always carries exactly one path — the cursor — and `paths` carries the whole selection beside it. |
| `get-root` | Print root |
| `set <setting> <value>` | Runtime toggles: hidden, ignored, compact, icons, search case, … |
| `set-root <dir>` | Re-root |
| `open` | Open current selection |
| `quit` | Exit |

- **No mutation verbs.** Rename/delete over a socket is a scripting foot-gun with no user story. Mutations are human-initiated.
- Security: socket dir `0700`; filesystem permissions are the auth model.
- Killer primitive: `nvim "$(birch ctl get-path)"` bound in the main pane is reverse integration with zero editor-specific code in birch.

## Picker mode

`birch --pick` — the same UI and the same search as the pane (ADR 0023): the tree stays,
non-matches dim and go inert, `↑`/`↓` step the matches. Enter prints the selected path —
file or dir — to stdout and exits. Arrows browse; a mouse click selects, only double-click
picks (chevrons browse dirs), so exploratory clicks never confirm by accident.

**The glob filter** (`--filter '*.md'`, repeatable) narrows what the tree offers, in either
mode. Patterns read as a shell or `.gitignore` reads them: no `/` matches the **name**, an
interior `/` the **root-relative path**, a **trailing** `/` names **directories** (so `*/` is
any directory). Non-matching **files** are dimmed and inert (`--filter-mode skip`, the default)
or omitted (`hide`). Directories are never dimmed and never hidden — the tree must stay walkable
to reach what matches, and with lazy loading "this branch is empty" only becomes known mid-browse,
which made rows vanish under the cursor — but a directory is **pickable** only when a pattern
names it. Search runs inside the filtered set.

```
nvim "$(birch --pick)"
cd "$(birch --pick)"
```

Mutations disabled by default in picker mode. This is the adoption funnel: picker first, persistent pane once hooked.

## Pane integration: host adapters (birch-herdr, birch-cmux, …)

The persistent-sidebar use case is the product; without integration, Enter opens `$EDITOR` *inside the 40-column birch pane*.

**No plugins in birch.** Integration lives host-side as a **thin adapter** — `birch-herdr`, `birch-cmux` — that starts birch with the right settings so the host can talk to it over the socket. birch's entire promise to adapters is: the socket protocol, `--socket`, `--open-cmd` with `--open-detached`, and clean SIGHUP exit. Nothing else.

An adapter's job (~a screen of code):

1. Spawn the side pane running `birch --socket <host-chosen-path> --open-cmd '<host open primitive> {}'`
2. Wire the toggle keybinding — host kills/respawns the pane; birch needs nothing
3. Wire reverse-reveal: host's file-focus hook → `birch ctl --socket <path> reveal <file>` — the tree follows your editing, IDE-style
4. Optionally: main-pane bindings for `birch ctl open` / `get-path`

**Adapters live with the host, not in the birch repo.** The host knows its own pane, keybinding, and hook APIs; birch stays free of N host integrations (same instinct as "no plugins"). The birch repo ships three *reference* adapters as documentation of the pattern — installed on `PATH` beside `birch` since [ADR 0027](adr/0027-adapters-ship-as-binaries.md), because a binding that cannot be named is not usable documentation — plus plain recipes for hosts without adapter hooks:

- **tmux**: pane spawn + `send-keys` open-cmd + toggle binding (recipe, not adapter — tmux has no hook surface worth wrapping).
- **cmux**: integrates via cmux's native right-sidebar **Dock** rather than a workspace split — cmux owns the lifecycle from `dock.json`; the adapter supplies a per-window socket, preview-as-tab, and a workspace-follow watcher (ADR 0016, rides the Dock beta).
- **Forward** (tree → editor) without any adapter: `--open-cmd` per target — `nvim --server $NVIM --remote`, `emacsclient`, `code -r`.
- **Reverse** without any adapter: two-line editor autocmd calling `birch ctl reveal <file>` on buffer switch.

Single-instance discipline is the host's job; document it, don't build lockfiles. Verify SGR mouse passthrough in herdr early (flagship host).

## Config

`~/.config/birch/birch.toml` for personal defaults (an always-running tool with flags-only config is hostile); CLI flags override; `birch ctl set` changes at runtime.

## Sequencing (scope cuts TBD)

Ordering reflects dependency + value, not release commitments; scope will be cut later.

| Phase | Contents |
|---|---|
| 0.1 | Tree, arrows, lazy load, icons, open-cmd, basic mouse (click/scroll), **real-tree/render split + source interface in place** |
| 0.2 | Git badges + ancestor propagation, live updates, compact folders |
| 0.3 | Filename fuzzy search (jump-style) |
| 0.4 | Control socket + `birch ctl` + `--socket`, `--pick`, birch-herdr reference adapter + nvim/tmux recipes, state persistence |
| 0.5 | File ops + context menu + hover + copy name/paths (OSC 52) |
| 0.6 | Content search source |
| Later | Git Changes source, Project View source, Open with…, config polish |

Guiding cut principle: pane integration beats features. A birch with flawless herdr/nvim reveal and no content search beats the reverse.

## Open questions

- Selection stability details during heavy churn (build systems generating files while user navigates).
- `set-root` above the original root — allowed, or only descend?
- Multi-instance on the same root: symlink points to most recent — good enough, or does `birch ctl` need instance listing?
- Trash on exotic filesystems / NFS — fallback behavior when trash is unavailable.
- Name availability: verify `birch` on crates.io / homebrew before attachment hardens.
