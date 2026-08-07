---
okf_version: "0.1"
---

# Process

* [Task workflow](workflow.md) - sessions, sprints, design/implementation approvals, ADRs.

# Sprints

* [Sprint 001 — Foundation — name check and the core tree view](sprints/sprint-001.md) - Done.
* [Sprint 002 — Live decorated tree — git status, live updates, compact folders](sprints/sprint-002.md) - Done.
* [Sprint 003 — Find things — fuzzy search, picker mode, state persistence](sprints/sprint-003.md) - Done.
* [Sprint 004 — Integration — control socket, birch-ctl, adapter and recipes](sprints/sprint-004.md) - Done.
* [Sprint 005 — MVP feedback — quit keys, root row, VISUAL](sprints/sprint-005.md) - Done.
* [Sprint 006 — Search feel — match what is displayed](sprints/sprint-006.md) - Done.
* [Sprint 007 — Second feedback batch — visuals, picker, CLI truth](sprints/sprint-007.md) - Done.
* [Sprint 008 — Third feedback batch — cmux adapter, chain splitting, {line} cleanup](sprints/sprint-008.md) - Done.
* [Sprint 009 — cmux adapter live-use refinements](sprints/sprint-009.md) - Done.
* [Sprint 010 — Click model and detached open commands](sprints/sprint-010.md) - Done.
* [Sprint 011 — cmux Dock integration](sprints/sprint-011.md) - Done.
* [Sprint 012 — Publishable repo & process docs](sprints/sprint-012.md) - Done.
* [Sprint 013 — Installable & CI-guarded](sprints/sprint-013.md) - Done.
* [Sprint 014 — Docs & publication polish](sprints/sprint-014.md) - Done.
* [Sprint 015 — Visual design: earn "beautiful"](sprints/sprint-015.md) - Done.
* [Sprint 016 — Navigation & search feel](sprints/sprint-016.md) - Done.
* [Sprint 017 — Pointer feel](sprints/sprint-017.md) - Done.

# Tasks

* [Encapsulate visual styles entirely in birch-tui](tasks/055-encapsulate-themes-in-tui.md) - Tech debt: remove ThemeId from core; DI seam so the render layer owns theme identities.
* [Support user-authored themes](tasks/056-add-user-themes.md) - Future: load custom themes from disk (~/.config/birch/themes/*.toml) beyond the built-in catalog.
* [Adapt themes to the terminal color scheme](tasks/058-adapt-themes-to-terminal-palette.md) - Themes assume black bg; make them respect the terminal palette (light/dark, base16).
* [Highlight the active folder's indent guide](tasks/061-active-path-indent-guide.md) - Opt-in theme axis: dim all guides except the current folder's column, which brightens; indent lines and connectors.
* [Make git badge placement configurable](tasks/064-configurable-badge-placement.md) - Theme axis + setting: right / left / none, with a fitting default per built-in theme.
* [Add a "random" theme](tasks/065-random-theme.md) - --theme random resolves to a randomly chosen built-in theme at launch.
* [Support animated gradient colours](tasks/066-animated-gradient-themes.md) - Moving colour bands (rainbow left-to-right); needs a frame clock, focus-aware pausing, strictly opt-in.
* [Add in/out badges for branch changes](tasks/078-add-branch-diff-badges.md) - Mark files whose changes are committed on this branch but not on main, and the reverse.
* [A quit arriving during a terminal handover is swallowed](tasks/077-quit-swallowed-during-terminal-handover.md) - perform_open's stale-event drain discards the quit flag, so SIGHUP or ctl quit is answered and ignored.
* [Search is unusable on a large root](tasks/076-search-unusable-on-large-roots.md) - From $HOME the index never lands and the status line claims "no matches"; both modes, not just --pick.
* [Show how much a filter actually matches](tasks/070-show-filter-match-counts.md) - Per-folder or total match counts, so a filtered browse knows where to go.
* [Add the context menu](tasks/071-add-context-menu.md) - The right-click action surface, split out of the 0.5 bundle so it can land before the ops it hosts.
* [Move files and folders by dragging](tasks/072-drag-to-move.md) - Drag a row onto a directory to move it; blocked on a scope-fence amendment, since drag-and-drop move is on the permanent out-of-scope list.
* [Show a hotkey reference](tasks/073-hotkey-reference.md) - In-app discoverability: always-on footer vs. summoned overlay, with `?` unavailable (printables are search).
* [Support multi-selection](tasks/074-add-multi-selection.md) - Shift/Ctrl range and toggle, mouse and keyboard; blocked on a scope-fence amendment, since multi-select is on the permanent out-of-scope list.
* [Support multiple roots](tasks/026-add-multiple-roots.md) - Sibling roots in one instance; needs design.
* [Add copy name and paths](tasks/028-add-copy-paths.md) - OSC 52 copy split out of the 0.5 bundle.


* [Add file operations](tasks/029-add-file-operations.md) - Rename/delete/new inline ops with git-aware delete; context menu split to 071, copy paths to 028.
* [Add the content search source](tasks/030-add-content-search.md) - Ctrl-F swaps the pane's source to files-with-matches, built on the ripgrep crates.
* [Add the Git Changes source](tasks/032-add-git-changes-source.md) - A third source listing changed files, reusing the source-as-delta-stream interface.
* [Add the Project View source](tasks/033-add-project-view-source.md) - A curated/virtual tree source, reusing the source-as-delta-stream interface.
* [Add "Open with…" to the context menu](tasks/034-add-open-with.md) - Choose an alternative open command for the selected node.
* [Verify the herdr integration live](tasks/035-verify-herdr-integration.md) - SGR mouse passthrough, open-in-main, toggle, reverse-reveal in a live herdr session.

# Publication

Pre-publication work — repo hygiene, distribution, and process docs. Not product features,
so outside the `docs/design.md` scope fence.

* [Install the contrib adapters on PATH](tasks/051-install-adapters-on-path.md) - brew install puts them in share/birch/, not PATH; make them callable by bare name (or document the path).
* [Add a flag to disable state persistence](tasks/053-add-state-persistence-toggle.md) - Turn off remembering/restoring expansion/selection/scroll per root.

# Done

* [Verify the name "birch" is available](archive/001-verify-name-availability.md) - Name kept per ADR 0002; crates.io conflict noted, Homebrew free.
* [Build the core tree view (phase 0.1)](archive/002-build-core-tree-view.md) - Tree, arrows, lazy load, icons, open-cmd with {line}, basic mouse, real-tree/render split and source interface.
* [Add git status badges](archive/003-add-git-status.md) - Badges, rollups, deleted-but-tracked rows, ignored dimming via the porcelain v2 side-table.
* [Add live filesystem and git updates](archive/004-add-live-updates.md) - Non-recursive watches per displayed dir; debounced re-scans; git refresh piggybacks.
* [Add compact folders](archive/005-add-compact-folders.md) - Flatten-time chains with viewport peek-loading (ADR 0007 as amended).
* [Add fuzzy filename search](archive/006-add-fuzzy-filename-search.md) - Jump-style search over an ignore-walk index; q types into search (ADR 0008).
* [Add picker mode](archive/007-add-picker-mode.md) - --pick/--pick-dir on stderr; stdout carries only the picked path.
* [Add state persistence](archive/008-add-state-persistence.md) - Expansion/selection/scroll per root, atomic writes, git-gated restore.
* [Add the control socket and birch-ctl](archive/009-add-control-socket.md) - NDJSON protocol (ADR 0011), addressing/lifecycle (ADR 0010), walk-up client.
* [Ship the reference host adapter and recipes](archive/010-add-host-adapter-and-recipes.md) - birch-tmux (live-verified) + birch-herdr adapters, integrations guide.
* [Esc backs out — and quits at top level](archive/011-fix-quit-keys.md) - Layered dismissal per ADR 0012; Ctrl-C always quits.
* [Show the root as the first tree row](archive/012-show-root-row.md) - Root as row zero, children nested, never chained.
* [Open defaults prefer VISUAL over EDITOR](archive/013-prefer-visual-editor.md) - VISUAL, then EDITOR, then the platform opener.
* [Match what is displayed — name-first search with highlighted characters](archive/014-refine-search-matching.md) - Names by default, path on /, lit match characters (ADR 0013).
* [Polish tree visuals — LICENSE icon, IDEA-style match boxes, root path](archive/015-polish-tree-visuals.md) - Feedback batch two.
* [One picker — Enter always picks](archive/016-unify-picker.md) - Single --pick; Enter confirms files and dirs alike.
* [CLI truth — --open-cmd help, --no-socket](archive/017-cli-truthfulness.md) - Honest help text; socket opt-out.
* [Ship the birch-cmux adapter](archive/018-add-cmux-integration.md) - Live-verified; Enter opens cmux preview tabs, one birch per workspace.
* [Split compact chains on demand](archive/019-split-chains-on-demand.md) - `→` splits an expanded chain; collapse re-fuses (ADR 0014).
* [Drop {line} from the open-cmd template](archive/020-drop-line-template.md) - `{}`-only contract; open-at-line moved to content search; backlog audit done.
* [Refine the cmux adapter after first live use](archive/021-refine-cmux-adapter.md) - Preview pane layout, focus discipline, side selection, mise PATH.
* [Click selects, double-click activates](archive/022-click-selects-first.md) - 450 ms path-keyed double-click; chevrons immediate (ADR 0015).
* [Detached open commands — --open-detached](archive/023-detach-open-cmd.md) - Fire-and-forget open-cmds; adapters flash-free.
* [Adopt the cmux Dock integration](archive/024-adopt-cmux-dock-integration.md) - birch-cmux rewritten around the Dock: per-window socket, preview-as-tab, workspace-follow watcher (ADR 0016).
* [Add the MIT LICENSE file](archive/036-add-license-file.md) - Root LICENSE (MIT), Copyright (c) 2026 Stanislav Lukyanov.
* [Remove tracked scratch fixtures and tighten .gitignore](archive/037-remove-scratch-fixtures.md) - Dropped bar.md / bar2.md / foo/; ignore .claude/ .cmux/ .readb.
* [Split the workflow doc into operational core and meta](archive/039-split-workflow-doc.md) - Reformulated hygiene gate (hygiene + voice), scope-presentation format, new bundle layout documented.
* [Define the external contribution flow](archive/040-define-contribution-flow.md) - Root CONTRIBUTING.md; standard PRs welcome, distinct from the workflow doc.
* [Restructure the tasks bundle](archive/041-restructure-tasks-bundle.md) - Numbered task slugs, closed tasks to tasks/archive/, sprint files to tasks/sprints/.
* [Add the CI workflow](archive/038-add-ci-workflow.md) - GitHub Actions: fmt --check, clippy --all-targets, test on Linux + macOS.
* [Set up the Homebrew tap and formula](archive/042-set-up-homebrew-tap.md) - slukyano/homebrew-tap; formula generated + pushed by cargo-dist (ADR 0018).
* [Automate tagged releases](archive/043-automate-releases.md) - cargo-dist release.yml: v* tag → 3-target build → GitHub Release → tap formula.
* [Fold the control client into a `birch ctl` subcommand](archive/050-unify-control-client.md) - birch-ctl folded into `birch ctl` (ADR 0019); one binary, one formula.
* [Document installation in the README](archive/044-document-installation.md) - Install + Quick Start sections: brew install, cargo install --git, from source, adapter caveat.
* [Add a demo recording to the README](archive/045-add-repo-demo.md) - vhs GIF opening on the tree with icons, git badges, and fuzzy search.
* [Fill in Cargo package metadata](archive/046-add-cargo-metadata.md) - repository / homepage / keywords / categories for discoverability.
* [Decide the crates.io publishing story](archive/047-decide-crates-io-publish.md) - Defer crates.io; Homebrew + cargo-install-from-git are the channels (ADR 0020).
* [Add a changelog](archive/048-add-changelog.md) - CHANGELOG.md (Keep a Changelog), [0.1.0] entry.
* [Deduplicate and route the documentation set](archive/049-dedup-and-route-docs.md) - Single home per topic across README / AGENTS / CONTRIBUTING / workflow.md / docs/; docs/ + backlog/ as OKF bundles; README overhaul, logo, badges.

* [Refine the tree's visual design — earn "beautiful"](archive/054-refine-tree-visual-design.md) - The theme engine (ADR 0021) + the birch flagship: silver bark, sage icons, depth-fading guides, one gold stroke.
* [Add visual styles](archive/025-add-visual-styles.md) - Eleven built-in themes: measured editor mimics, official-palette schemes, the Commander, plain; per-theme icon families.
* [Add the config file](archive/031-add-config-file.md) - ~/.config/birch/birch.toml (ADR 0022): theme + toggles + open-cmd; bidirectional flags; config < flags < ctl set.
* [Canonicalize symlinks before the reveal root-containment check](archive/052-fix-reveal-symlink-canonicalization.md) - reveal matches as-given, canonicalizes as fallback; /tmp works on macOS.
* [Remove the files-first setting](archive/057-remove-files-first.md) - Setting, flag, and protocol key dropped; directories always sort first.
* [Search match cycling walks the tree](archive/063-search-cycles-in-tree-order.md) - Matches held in tree order; the selection anchors forward on every keystroke.
* [One search model — the picker keeps the tree](archive/062-unify-search-in-pick-mode.md) - Flat hit list deleted; dimmed rows are inert (ADR 0023).
* [Add the glob view filter](archive/027-add-picker-filter.md) - `--filter`/`--filter-mode` in both modes; files judged, folders navigable but gated on pick.
* [→ always moves or reveals](archive/060-right-arrow-always-advances.md) - Expand, split, or advance; inert only when no live row follows.
* [Indent guides vs. chevrons: a fallback-font hazard](archive/059-fix-guide-chevron-alignment.md) - Measured to the font files; documented, no render change.
* [Input bursts freeze the pane](archive/069-fix-wheel-scrolling.md) - One event cost a full O(rows) rebuild behind an unbounded queue; the loop now draws once per batch (ADR 0024).
* [Make scroll speed configurable](archive/075-configurable-scroll-speed.md) - Rows per wheel tick as a setting (1-10, default 3), across flag, config key, and ctl set.
* [Select on mouse release, not on press](archive/067-select-on-mouse-up.md) - A click is a press and a release on the same row and zone, acting on the release (ADR 0025).
* [Add a scrollbar](archive/068-add-scrollbar.md) - One inert column at the right edge, shown only when the rows overflow.

# Dropped
