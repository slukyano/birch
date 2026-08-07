# Task Bundle Log

## Earlier (predates the log-dating convention)

* **Initialization**: Created the `tasks/` OKF bundle, the [task workflow](workflow.md),
  and the first draft tasks seeded from the sequencing plan in
  [the design doc](../docs/design.md).
* **Backlog audit**: Compared the backlog against the design doc; added draft tasks for the
  config file and the "Later" pool (Git Changes source, Project View source, Open with…);
  folded the defaults-table flags and the design doc's open questions into the relevant task
  bodies. Recorded the MVP boundary in
  [ADR 0001](../docs/adr/0001-mvp-scope.md): phases 0.1–0.4 are the MVP.
* **Sprint 001 done**: Name verified and kept ([ADR 0002](../docs/adr/0002-keep-the-name-birch.md));
  phase 0.1 core tree view implemented across the three crates per ADRs 0003/0004,
  reviewed independently, review findings fixed.
* **Sprint 002 done**: Git badges ([ADR 0005](../docs/adr/0005-git-status-via-git-cli.md)),
  live updates ([ADR 0006](../docs/adr/0006-snapshot-deltas-stateless-sources.md)), and
  compact folders ([ADR 0007](../docs/adr/0007-compaction-peek-loading.md), amended) landed;
  independent review found two blockers (ignored-flag semantics, symlink peek loops), both fixed.
* **Sprint 003 done**: Fuzzy search ([ADR 0009](../docs/adr/0009-search-index-and-engine.md),
  [ADR 0008](../docs/adr/0008-q-types-into-search.md)), picker mode, and state persistence
  landed; review applied nine fixes and added the app-layer test suite.
* **Sprint 004 done — MVP complete**: Control socket + birch-ctl
  ([ADR 0010](../docs/adr/0010-socket-addressing-and-lifecycle.md),
  [ADR 0011](../docs/adr/0011-ndjson-protocol.md)) and the reference adapters/recipes
  landed; security review fixed two injection blockers in the adapter scripts. All ten
  MVP tasks of [ADR 0001](../docs/adr/0001-mvp-scope.md) are Done.
* **Sprint 005 done**: First-use feedback applied — Esc backs out and quits at top level
  ([ADR 0012](../docs/adr/0012-esc-backs-out.md)), the root renders as the first tree row,
  and open defaults prefer `$VISUAL`. Repository published (private) to GitHub.
* **Enter toggles dirs**: Maintainer decision — Enter on an expanded dir collapses it
  (VS Code behavior); the keyboard table now lists `→` (expand) and `Enter` (toggle)
  separately. Files still always open on Enter.
* **Sprint 006 done**: Search matches what is displayed
  ([ADR 0013](../docs/adr/0013-match-what-is-displayed.md)) — names by default, paths on
  `/`, matched characters highlighted; review caught and fixed nucleo's non-char index
  units for Unicode names.
* **Sprint 007 done**: Feedback batch two — LICENSE icons, IDEA-style match boxes, root
  path annotation, the unified Enter-always-picks picker, honest --open-cmd help, and
  --no-socket. Four future drafts seeded (visual styles, multiple roots, picker filter,
  copy paths). The chain-arrow report is awaiting a repro from the maintainer.
* **Sprint 008 scoped**: Feedback batch three — the design doc's planned `birch-cmux`
  adapter (the maintainer now works in cmux), chain splitting on `→`
  ([ADR 0014](../docs/adr/0014-chains-split-on-demand.md); the earlier chain-arrow
  report was a feature request, not a bug), and dropping the premature `{line}`
  placeholder in favor of the content-search task owning the open-at-line contract.
* **Sprint 008 done**: Feedback batch three — `contrib/birch-cmux` live-verified in the
  maintainer's cmux session (Enter opens files as cmux preview tabs; one birch per
  workspace), `→` splits expanded compact chains and collapse re-fuses them
  ([ADR 0014](../docs/adr/0014-chains-split-on-demand.md)), and the `{line}` placeholder
  is gone from the open-cmd contract (stale templates fail loudly; open-at-line now
  belongs to the content-search task). One cmux crash during testing diagnosed as an
  upstream stale-process bug, not birch.
* **Sprint 009 scoped**: Live-use feedback on the cmux adapter, design direction from
  the maintainer in chat — clean socket path, no focus stealing, selectable tree side,
  a dedicated preview pane (master → preview → tree), mise-provided in-repo PATH, and
  the socket kept (reverse-reveal/scripting only). New convention: cmux debugging runs
  in a separate instance, never the maintainer's.
* **Sprint 009 done**: cmux adapter live-use refinements — dedicated preview pane
  (master | preview | tree, side selectable via `BIRCH_CMUX_SIDE`), no focus stealing,
  clean socket paths, and mise-provided in-repo PATH (`birch`, `birch-cmux` work bare
  inside the repo after `cargo build`). Review hardened the tree parsers (refs anchored
  to node-type words) and the preview bootstrap error path. The socket stays: it serves
  reverse-reveal and scripting only.
* **Sprint 010 scoped**: Second live-use feedback batch, both decisions made by the
  maintainer in chat — click selects (double-click activates; chevron and Enter
  unchanged), and `--open-detached` marks a custom open-cmd as fire-and-forget so
  adapter opens stop suspending the TUI (the tree-pane flash). Alongside, outside the
  sprint: the in-repo PATH patch moved behind an explicit `mise run dev` subshell.
* **Sprint 010 done**: Click model and detached open commands — a single click now only
  selects (450 ms path-keyed double-click activates; chevrons toggle immediately;
  ADR 0015 reverses the VS Code-school rule), and `--open-detached` runs fire-and-forget
  open-cmds with null stdio so adapter previews stop suspending the tree pane (all three
  contrib adapters pass it). Review hardened the picker filter list (a single chevron-zone
  click could confirm a pick) and hit_test on missing dirs. Fold-ins: one folder glyph
  (chevron alone carries expansion state) and the `mise run dev` build+subshell replacing
  the always-on in-repo PATH patch. Merged to mvp, and mvp to main on maintainer
  instruction.
* **Sprint 011 done**: cmux Dock integration — `contrib/birch-cmux` rewritten around
  cmux's right-sidebar Dock ([ADR 0016](../docs/adr/0016-cmux-integrates-via-the-dock.md)),
  replacing the workspace-split adapter that had irreducible open flicker. Three verbs
  (`dock-run`, `preview`, `dock-socket`), one birch per window keyed on the window id, a
  follow watcher that re-roots on `workspace.selected` and dies with the window or cmux.
  Previews open as tabs in the main pane (no split). Dock-only, no non-dock fallback;
  tmux/herdr keep the split-pane pattern. No birch/Rust changes — uses existing
  `--socket`/`--open-cmd`/`--open-detached`/`birch-ctl set-root`. Rides the Dock beta.
  Executed autonomously and consolidated onto `main` (retiring the `mvp` branch) per
  maintainer direction.
## 2026-07-24

* **Publication prep**: Collapsed development history into a single `prepare for
  publication` root commit and stripped authoring dates from the `tasks/` and `docs/adr/`
  bundles (sprints kept, dates removed; the workflow no longer stamps dates). Pushed to a
  new public GitHub repo (the old private repo renamed to `birch-private` as a backup).
  Seeded a `# Publication` backlog — twelve `Draft` tasks spanning repo hygiene (LICENSE,
  scratch-fixture removal, CI), distribution (Homebrew tap, release automation, install
  docs), external polish (README demo, Cargo metadata, crates.io decision, changelog), and
  process docs (splitting `workflow.md` into operational core vs. skill meta, and defining
  an external contribution flow).
* **Sprint 012 done**: Publishable repo & process docs. Root `LICENSE` (MIT) and
  `CONTRIBUTING.md` (standard PRs welcome, distinct from the maintainer's sprint flow) added;
  scratch fixtures removed and `.gitignore` tightened; `workflow.md` reworked — the
  publication-hygiene gate reformulated into hygiene + voice, a scope-presentation format
  added, and the new bundle layout documented. Bundle restructured: task files renamed to
  `NNN-slug.md` with the number in the concept name, closed tasks moved to `tasks/archive/`,
  sprint records to `tasks/sprints/`, and every `blocked_by` / sprint `tasks:` reference
  renumbered. The distribution and polish tasks (CI, Homebrew, releases, install docs, demo,
  Cargo metadata, crates.io decision, changelog) stay `Draft` for a later infrastructure sprint.
* **Bundle relocated**: `tasks/` renamed to `backlog/`, with active task concepts moved under `backlog/tasks/`; the bundle root now holds only `index.md`, `log.md`, and `workflow.md` (plus the `tasks/`, `archive/`, and `sprints/` subdirectories). Fixed the relative `../docs` links in the archived and sprint files that the sprint-012 restructure had left one level too shallow. A structural change, made directly on `main`.
* **Docs-architecture task seeded**: `049-dedup-and-route-docs` (`Draft`, high) — deduplicate and route each topic to a single home across `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `backlog/workflow.md`, and `docs/`, with progressive-disclosure pointers; add a `docs/` index (and log); and date the change logs. Design-heavy, touches binding docs — for a sprint.
* **Sprint 013 done — birch v0.1.0 shipped**: Installable & CI-guarded. GitHub Actions CI
  (`fmt`/`clippy`/`test` on Linux + macOS); release automation and the Homebrew tap adopted via
  **cargo-dist** ([ADR 0018](../docs/adr/0018-release-via-cargo-dist.md), supersedes 0017) after
  the "cargo-dist is unmaintained" premise proved false. The pivot forced folding `birch-ctl` into
  a `birch ctl` subcommand ([ADR 0019](../docs/adr/0019-control-client-is-a-birch-subcommand.md);
  one binary → one formula). **v0.1.0** is published — public GitHub Release (three platforms) and
  `Formula/birch.rb` in `slukyano/homebrew-tap`; `brew install slukyano/tap/birch` resolves and
  verifies. Deferred to a later docs sprint: `044-document-installation`, `049-dedup-and-route-docs`,
  and the `045`–`048` polish tasks. Known edge: the contrib adapters install to `share/birch/`,
  not `PATH`.

## 2026-07-26

* **Sprint 014 done — docs & publication polish**: The documentation set was routed to one home per
  topic and `docs/` + `backlog/` became OKF bundles with an index and dated logs
  (`049-dedup-and-route-docs`); the README gained Install and Quick Start sections
  (`044-document-installation`) and a vhs demo GIF (`045-add-repo-demo`); Cargo `keywords`/`categories`
  were filled (`046-add-cargo-metadata`); crates.io was deferred in favor of Homebrew and
  `cargo install --git` ([ADR 0020](../docs/adr/0020-defer-crates-io.md), `047-decide-crates-io-publish`);
  and a `CHANGELOG.md` was added (`048-add-changelog`). The README was overhauled around a `# birch`
  H1 with an outlined birch logo, CI/release/license badges, a Features list, a Quick-Start-first
  order, the tagline "modern interactive file tree for the terminal", and a dedicated advanced
  `birch ctl` section. Three follow-ups were seeded from a fresh-eyes DX review — `052` (reveal
  symlink canonicalization), `053` (state-persistence toggle), `054` (tree visual polish) — and
  `025` was rescoped to style presets only. The many small doc commits were squashed before an
  independent diff review and the merge to `main`.

## 2026-07-28

* **Sprint 015 done — themes shipped, v0.1.1 released**: The render layer became a theme system
  ([ADR 0021](../docs/adr/0021-theme-system.md)) with eleven built-in themes — the `birch`
  flagship ("silver bark with a single gold stroke", depth-fading guides, sage-tinted icons),
  editor mimics measured from the real VS Code / IDEA / Xcode on-screen, five official-palette
  scheme themes, the Commander retro (canonical CGA), and the ANSI-safe `plain` — produced
  through a research workshop (glyph codepoints verified from tool sources; TUI-design and
  app-tree surveys) and two adversarial design-review rounds, with the "semantics global, hues
  local" rule enforced in the engine. The config file landed
  ([ADR 0022](../docs/adr/0022-config-file.md), `054`/`025`/`031`), `reveal` handles symlinked
  prefixes (`052`), `files-first` was removed (`057`), and the research was preserved under
  `docs/research/`. Follow-ups live as `055` (tui encapsulation), `056` (user themes), `058`
  (terminal-palette adaptation). Released as **v0.1.1**.
* **Two bug reports filed**: `059` — indent guides read as misaligned because non-`Mono` Nerd Font
  PUA glyphs render wider than one cell and shift right (measured: guide centred at the cell
  centre, chevron ink +5 px in a 16 px cell; pixel-exact in a `Mono` variant). `060` — `→` is a
  silent no-op on files and on already-expanded plain directories; it should descend into a folder
  or advance to the next row, doing nothing only on the tree's last file.
* **Active-guide feature seeded**: `061` — an opt-in theme axis dimming every indent guide except
  the current folder's column, which brightens (current folder = the parent for files and collapsed
  dirs, the directory itself when expanded). Covers both indent lines and classic connectors, and
  takes up the guide-ancestry highlighting the theme engine left open.
* **Search/picker bugs and theme features filed**: `062` (picker mode replaces the tree with a flat
  match list and disables `→`/`←`, diverging from tree-mode search), `063` (`↑`/`↓` cycle matches in
  fuzzy-score order — `search.rs` sorts by score — so the selection teleports; should walk tree
  order), `064` (badge placement as a theme axis + setting: right / left / none, defaults grounded
  in the measured editors; left placement moves `hit_test` geometry), `065` (a `random` theme), and
  `066` (animated gradient colours — blocked less by colour maths than by birch having no frame
  clock; needs an opt-in tick and focus-aware pausing). `027` (picker filter) was expanded with the
  repeatable-glob spelling, `hide`/`skip` modes, and the folders-navigable-but-not-pickable rule.

## 2026-07-28 (sprint 016)

* **Sprint 016 done — one search model, and dimming that means something**:
  [ADR 0023](../docs/adr/0023-narrowing-dims-and-dimmed-is-inert.md) replaced two narrowing
  behaviours with one rule — a search or a filter **dims** rows instead of replacing them, and a
  **dimmed row is inert**: it cannot hold the selection, a click does nothing, `Enter` never acts on
  it. Each narrowing declares what it judges: a search judges every row, so a directory failing the
  query dims too; the glob filter judges files only, so directories stay navigable and are gated on
  *pick* alone. The picker's flat hit list is deleted (`062`), so `--pick` and the pane are the same
  tree, the same search, the same keys. Search matches are held in **tree order** and the selection
  **anchors forward** on every keystroke — the first match at or after the current row, wrapping,
  staying put while the row still matches (`063`). `→` gained a three-case rule: expand, split, or
  advance, inert only when no live row follows (`060`). A **glob filter** landed as `--filter`
  (repeatable) and `--filter-mode hide|skip`, in both modes, with search's corpus rule — name
  without `/`, root-relative path with one (`027`).
* **Two rendering bugs found by looking at the screen**: the edge-to-edge selection wash overwrote
  every background in its row, including the gold match highlight, so the lit characters of the
  current match rendered dark-on-dark and vanished — on precisely the row the selection sits on; and
  a dim row kept `bold_dirs`, so an inert directory still read as prominent. Both fixed, the first
  with a `TestBackend` regression test.
* **The indent-guide report was measured, not guessed** (`059`): a `vhs` capture harness plus
  `fontTools` on the actual font files showed cmux embeds an unpatched JetBrains Mono — which
  carries **no** Nerd Font codepoint — beside **Symbols Nerd Font**, where birch's chevrons advance
  1.67 cells and sit +0.42 cell off centre while the indent guide, drawn by the primary font, sits
  at +0.00. No birch change: the fix is a `Mono` primary family, now documented in the README and
  [the glyph reference](../docs/research/nerd-font-glyphs.md) with the upstream Ghostty reports.

## 2026-07-29

* **Four seeds from the maintainer, two of them against the fence**: `071` splits the **context
  menu** out of the phase-0.5 bundle (following `028`, which had already taken copy-paths), so the
  action surface can land before the ops it hosts; `029` narrows to the op layer accordingly.
  `073` asks for an in-app **hotkey reference** and exists to decide always-on footer vs. summoned
  overlay — sharpened by the fact that `?`, the conventional summon key, is a search character and
  can never be bound. `072` (**move by dragging**) and `074` (**multi-selection**) are both named on
  the *permanently out of scope* list in [the design doc](../docs/design.md) — "drag-and-drop move",
  "multi-select", "bulk operations" — so each records the conflict rather than assuming it away:
  neither is designable until the scope fence is amended, which is a maintainer decision and
  warrants an ADR. `074` sketches the defensible middle stop (read-only plurality: multi-select
  feeds copy-paths and `--pick`, mutations stay single-target).

## 2026-08-06 (sprint 017)

* **The pointer sprint, and the loop underneath it.** `069` was filed as a wheel defect with
  peek-loading as the suspect; a PTY harness feeding synthetic wheel events and timing how long a
  keystroke waits behind a burst disproved that — peeks were ~18 %, git none, and 1 000 `Down`
  keypresses froze the pane identically. The defect was one full row rebuild per event, twice per
  input event, behind an unbounded queue. The loop now handles a **batch** and draws once
  ([ADR 0024](../docs/adr/0024-the-loop-draws-once-per-batch.md)); the task was retitled *Input
  bursts freeze the pane*. A hard flick went from 785 ms unresponsive to single-digit ms.
* **A click now completes on release** ([ADR 0025](../docs/adr/0025-a-click-completes-on-release.md)),
  on the row and zone it started on, so sliding off revokes it — settled here rather than inside
  `071`/`072`, which both build on the answer.
* **Two new settings**: `--scroll-lines` (`075`, added mid-sprint at the maintainer's request) and
  the `068` scrollbar toggle, each with a config key and a `ctl set` key.
* **Two bugs filed, not fixed**: `076` — search is unusable from a home-sized root, and the status
  line claims "no matches" whenever the index is merely absent; `077` — a quit arriving during a
  terminal handover is swallowed, found by the independent review in code outside the diff.

