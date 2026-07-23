# birch

Lean and beautiful interactive file tree for the terminal.

## Identity

birch does exactly what an IDE file tree does, and nothing else.

It is the IDEA / VS Code explorer panel, living in a terminal pane next to your editor. That sentence is both the feature list and the scope fence:

**In scope (the whitelist):** tree view, icons, git status, open file, fuzzy find, content search, rename / delete / new file / new dir, copy name/path, compact folders, live updates, mouse.

**Permanently out of scope:** miller columns, verbs or command language, vim modes, multi-select, bulk operations, copy/duplicate files, drag-and-drop move, search-and-replace, archive browsing, permissions editing, plugin API.

Contrast with neighbors:

- Not **yazi** — a tree, not miller columns.
- Not **broot** — no verbs, no modal behavior. Dead simple: arrows navigate, typing searches, Enter opens.
- Not **eza** — interactive and live, not a one-shot print.

birch is designed to run persistently in a side pane (herdr, cmux, tmux) — the pane integration story is a first-class concern, not an afterthought.

## Invocation

```
birch [<options>] [<dir>]
```

`<dir>` defaults to cwd and becomes the tree root. The root is fixed for the life of the instance except via `Reveal Root Here` (context menu) or `birch-ctl set-root`.

```
birch --pick [<dir>]      # picker mode: Enter prints the selection and exits
birch-ctl <verb> [...]    # control a running instance (see Control socket)
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
  - Keyboard treats a chain as one node (the tail) — until `→` on an expanded chain **splits** it into individual member rows; collapsing a member re-fuses the chain from that point (ADR 0014). Splits are session-local. Mouse segment-clicks target individual dirs (matters for context-menu scoping).
  - F2 on a chain inline-edits the full `a/b/c` fragment (this is rename-with-path reused).
  - Live updates split/fuse chains in place as children appear/disappear.

## Defaults

| Setting | Default | Initial flag |
|---|---|---|
| Icons | on | `--no-icons` |
| Git status | on | `--no-git` |
| Directories first | on | `--files-first` |
| Gitignored files | shown, dimmed, **auto-collapsed** | `--hide-ignored` |
| Hidden (dot) files | shown | `--hide-hidden` |
| Noise (`.git`, `.DS_Store`, …) | hidden | `--show-noise` |
| Compact folders | on | `--no-compact` |
| Mouse | on | `--no-mouse` |

Flags set initial values; runtime changes go through `birch-ctl set` (and possibly a few hotkeys later — added sparingly).

Ignored dirs (e.g. `node_modules`) are never auto-expanded, never searched, and never recursively watched.

## Interaction

### Keyboard

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection |
| `→` on dir | Expand; on an expanded chain, split it into member rows |
| `Enter` on dir | Toggle expand/collapse |
| `←` on dir / into parent | Collapse / jump to parent |
| `Enter` on file | Open (see Opening files). **Enter always opens — never contextual.** |
| any printable char | Fuzzy filename search |
| `Esc` | Back out: clear search / close menu; at top level, quit |
| `F2` | Rename (inline) |
| `Delete` | Delete (to trash) |
| `Ctrl-N` | New file / dir |
| `Ctrl-F` | Content search source |
| `Ctrl-Shift-C` | Copy relative path |
| `Ctrl-C` | Quit (`q` is a search character like any other) |

Printable characters are permanently reserved for search — no letter hotkeys, ever. The context menu is the primary surface for everything else; hotkeys above are accelerators shown in the menu.

### Mouse (native, on by default)

- **Single-click selects; double-click activates** (ADR 0015 — reversed from the original VS Code-school single-click-activates). In a host pane, a click meant to focus the pane — or just to point — arrives as a normal click, so the first click must be harmless: it only moves selection. A second click on the same row within 450 ms is Enter's twin (open file / toggle dir). Clicking a dir's chevron still toggles immediately without moving selection; each chevron press is its own toggle.
- **Scroll wheel**: 3 lines/tick.
- **Hover highlight** (SGR 1003 motion tracking).
- **Right-click → context menu** (keyboard works inside it: arrows/Enter/Esc):

```
Open
Open with…            (later)
──────────
New File…             ^N
Rename…               F2
Delete                ⌦
──────────
Copy Name
Copy Relative Path    ^⇧C
Copy Absolute Path
──────────
Reveal Root Here
```

- Right-click below the tree → menu targeting the root.
- **Text-selection trade-off**: mouse capture disables native drag-to-copy. Mitigations: Shift-click passes through to the terminal (documented in `--help`); Copy Path in the menu covers the real use case.
- No drag-and-drop. Inside tmux, mouse requires tmux mouse mode (FAQ, not a bug).

### Opening files

`Enter` / double-click runs the open command. Default: `$VISUAL {}` (else `$EDITOR {}`) if set, else `open` (macOS) / `xdg-open` (Linux).

```
--open-cmd '<template>'     # {} = path (appended when absent)
--open-detached             # the template is fire-and-forget: spawned with
                            # null stdio, no terminal handover (adapters)
```

A custom template hands the terminal over and waits (terminal editors) unless `--open-detached` marks it detached from the tty — the mode adapter open-cmds need, since they are short scripts driving the host's CLI, not tty programs.

Examples: `nvim {}`, `code -r {}`, or a host adapter's open primitive (herdr types into the main pane; cmux renders previews in a dedicated pane). The template grows a `{line}` placeholder when content search lands (the feature that produces line numbers); until then the contract is `{}`-only.

## Search

### Filename fuzzy search (just type)

- **Jump, not filter**: non-matches dim, matches highlight; `↑`/`↓` cycles matches; tree stays spatially stable (it's an ambient pane you glance at constantly). `Esc` restores.
- Whole-tree scope, auto-expanding to reveal matches, **never descending into ignored dirs**.
- Matches **simple names**; a query containing `/` matches full root-relative paths
  instead. Matched characters highlight in place — including inside compacted labels
  (`a/b/c`). Uppercase query characters anchor to capitals (camel humps).
- In `--pick` mode, search *filters* instead (transient picker → density wins). Same engine, two render policies.

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
- Smart-case on; no regex/case/word toggles in the UI (available via `birch-ctl set`).
- Respects the ignore boundary. Live updates re-search touched files only.
- **No search-and-replace, ever** (mutation-across-files; different product).

## File operations

Exactly four: **rename, delete, new file, new dir.** No move (see below), no copy, no multi-select, no op-history/undo stack.

- **Rename (F2)**: inline row editing — the selected row becomes a text input (never a bottom command bar; prompt bars grow verbs). Editing to a path (`../other/name.ts`) silently performs a move — rename-with-path is the 80% of "move" without the complexity cliff. Plain `rename()`; no `git mv` magic (git rename detection handles it).
- **New file (Ctrl-N)**: inline input in the target dir. `foo/bar/baz.ts` creates intermediate dirs; trailing slash (`foo/`) creates a dir. One binding, three ops (VS Code's trick). On a compacted chain, creates in the tail. No dedicated new-dir key.
- **Delete (⌦)**: **to trash** (macOS trash / XDG trash spec) — undoable beats confirmable. Confirmation only where trash can't help, and **git-aware**: deleting a *tracked* file is nearly free (recoverable from repo, shows as deleted badge — the git integration doubles as a safety net); deleting an *untracked* file gets the friction and says why.
- Ops and the watcher share the tree-delta stream: the op layer suppresses/owns watcher events for paths it is mutating (no flicker, no selection jumps mid-edit).
- Mutations are **human-initiated only** — no socket verbs, and disabled by default in `--pick` mode.

## Copy name / paths

Three variants (context menu; relative path also on `Ctrl-Shift-C`):

- **Copy Name** — `NewThing.tsx`
- **Copy Relative Path** — relative to the **birch root** (not cwd; root-relative is what you're looking at)
- **Copy Absolute Path**

Raw paths, no quoting/escaping magic (guessing the paste context is a losing game). On a compacted chain: tail's path; segment-click reaches intermediates.

**Clipboard mechanics**, in fallback order:

1. **OSC 52** — terminal sets the clipboard; works over SSH and inside tmux (`set-clipboard on`); native in ghostty. First because the flagship environment is exactly where subprocess clipboards fail silently.
2. Native fallback — `pbcopy` / `wl-copy` / `xclip`.
3. Last resort — print to status line.

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
  - `birch` — binary: wiring, flags, socket server.
  - `birch-ctl` — thin socket client.
  - Publishing these as a library ("libbirch") is deliberately deferred: no named external consumer exists, and SemVer stability would tax exactly the interfaces (sources, deltas) most likely to change. Revisit if a concrete embedder appears — the live candidate is herdr wanting the tree in-process rather than as a pane. Extraction from well-factored crates is cheap; un-publishing an API is not.
- **State persistence** (nice-to-have, cheap): expansion, selection, scroll per root at `~/.cache/birch/<root-hash>.json`; restored on launch. Crash/reboot resilience.
- Actions (open, rename, copy-path, …) live in one action layer consumed by hotkeys, mouse, context menu, and socket alike. Menu-specific logic is a smell.

## Control socket

A running birch instance exposes a Unix domain socket; `birch-ctl` is the thin client.

- **Addressing**: socket per instance at `$XDG_RUNTIME_DIR/birch/<pid>.sock`; per-root symlink `by-root/<root-hash>.sock` → most recent instance. `birch-ctl` resolves by walking up from cwd to a matching root — `birch-ctl get-path` in a sibling pane just works.
- **Host-dictated rendezvous**: `birch --socket <path>` binds exactly there, skipping default addressing (`--no-socket` skips binding entirely). A host that spawns birch shouldn't have to *discover* what it created — it picks the path and already knows it. `birch-ctl --socket <path>` targets it. This is the load-bearing flag for host adapters.
- **Lifecycle**: graceful exit on SIGHUP/SIGTERM (host closes pane → clean death, socket unlinked).
- **Protocol**: newline-delimited JSON request/response with a `v` field. **Additive-only evolution; clients tolerate unknown fields.** Host adapters may be maintained out-of-tree on their own release cadence — the protocol is a public API commitment, not an internal detail.
- **Verbs** (closed set — the whitelist applies to the protocol too):

| Verb | Effect |
|---|---|
| `reveal <path>` | Expand to and select path (editor → tree reverse integration) |
| `get-path [--name\|--rel\|--abs]` | Print current selection |
| `get-root` | Print root |
| `set <setting> <value>` | Runtime toggles: hidden, ignored, compact, icons, search case, … |
| `set-root <dir>` | Re-root |
| `open` | Open current selection |
| `quit` | Exit |

- **No mutation verbs.** Rename/delete over a socket is a scripting foot-gun with no user story. Mutations are human-initiated.
- Security: socket dir `0700`; filesystem permissions are the auth model.
- Killer primitive: `nvim "$(birch-ctl get-path)"` bound in the main pane is reverse integration with zero editor-specific code in birch.

## Picker mode

`birch --pick` — same UI as a transient fuzzy picker: search filters (not jumps), Enter
prints the selected path — file or dir — to stdout and exits. Arrows browse; a mouse
click selects, only double-click picks (chevrons browse dirs), so exploratory clicks
never confirm by accident.

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
3. Wire reverse-reveal: host's file-focus hook → `birch-ctl --socket <path> reveal <file>` — the tree follows your editing, IDE-style
4. Optionally: main-pane bindings for `birch-ctl open` / `get-path`

**Adapters live with the host, not in the birch repo.** The host knows its own pane, keybinding, and hook APIs; birch stays free of N host integrations (same instinct as "no plugins"). The birch repo ships one *reference* adapter as documentation of the pattern, plus plain recipes for hosts without adapter hooks:

- **tmux**: pane spawn + `send-keys` open-cmd + toggle binding (recipe, not adapter — tmux has no hook surface worth wrapping).
- **cmux**: integrates via cmux's native right-sidebar **Dock** rather than a workspace split — cmux owns the lifecycle from `dock.json`; the adapter supplies a per-window socket, preview-as-tab, and a workspace-follow watcher (ADR 0016, rides the Dock beta).
- **Forward** (tree → editor) without any adapter: `--open-cmd` per target — `nvim --server $NVIM --remote`, `emacsclient`, `code -r`.
- **Reverse** without any adapter: two-line editor autocmd calling `birch-ctl reveal <file>` on buffer switch.

Single-instance discipline is the host's job; document it, don't build lockfiles. Verify SGR mouse passthrough in herdr early (flagship host).

## Config

`~/.config/birch/birch.toml` for personal defaults (an always-running tool with flags-only config is hostile); CLI flags override; `birch-ctl set` changes at runtime.

## Sequencing (scope cuts TBD)

Ordering reflects dependency + value, not release commitments; scope will be cut later.

| Phase | Contents |
|---|---|
| 0.1 | Tree, arrows, lazy load, icons, open-cmd, basic mouse (click/scroll), **real-tree/render split + source interface in place** |
| 0.2 | Git badges + ancestor propagation, live updates, compact folders |
| 0.3 | Filename fuzzy search (jump-style) |
| 0.4 | Control socket + `birch-ctl` + `--socket`, `--pick`, birch-herdr reference adapter + nvim/tmux recipes, state persistence |
| 0.5 | File ops + context menu + hover + copy name/paths (OSC 52) |
| 0.6 | Content search source |
| Later | Git Changes source, Project View source, Open with…, config polish |

Guiding cut principle: pane integration beats features. A birch with flawless herdr/nvim reveal and no content search beats the reverse.

## Open questions

- Selection stability details during heavy churn (build systems generating files while user navigates).
- `set-root` above the original root — allowed, or only descend?
- Multi-instance on the same root: symlink points to most recent — good enough, or does `birch-ctl` need instance listing?
- Trash on exotic filesystems / NFS — fallback behavior when trash is unavailable.
- Name availability: verify `birch` on crates.io / homebrew before attachment hardens.
