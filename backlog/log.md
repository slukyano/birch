# Task Bundle Log

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
