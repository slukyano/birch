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

# Tasks

* [Add visual styles](tasks/025-add-visual-styles.md) - default / vscode / plain presets.
* [Support multiple roots](tasks/026-add-multiple-roots.md) - Sibling roots in one instance; needs design.
* [Add a picker file filter](tasks/027-add-picker-filter.md) - Glob/regex corpus restriction for picker mode.
* [Add copy name and paths](tasks/028-add-copy-paths.md) - OSC 52 copy split out of the 0.5 bundle.


* [Add file operations, context menu, and copy paths](tasks/029-add-file-operations.md) - Rename/delete/new inline ops, right-click context menu, hover highlight, copy name/paths over OSC 52.
* [Add the content search source](tasks/030-add-content-search.md) - Ctrl-F swaps the pane's source to files-with-matches, built on the ripgrep crates.
* [Add the config file](tasks/031-add-config-file.md) - Personal defaults in ~/.config/birch/birch.toml; CLI flags override; birch-ctl set changes at runtime.
* [Add the Git Changes source](tasks/032-add-git-changes-source.md) - A third source listing changed files, reusing the source-as-delta-stream interface.
* [Add the Project View source](tasks/033-add-project-view-source.md) - A curated/virtual tree source, reusing the source-as-delta-stream interface.
* [Add "Open with…" to the context menu](tasks/034-add-open-with.md) - Choose an alternative open command for the selected node.
* [Verify the herdr integration live](tasks/035-verify-herdr-integration.md) - SGR mouse passthrough, open-in-main, toggle, reverse-reveal in a live herdr session.

# Publication

Pre-publication work — repo hygiene, distribution, and process docs. Not product features,
so outside the `docs/design.md` scope fence.

* [Add the CI workflow](tasks/038-add-ci-workflow.md) - GitHub Actions: fmt --check, clippy --all-targets, test on push/PR.
* [Set up the Homebrew tap and formula](tasks/042-set-up-homebrew-tap.md) - slukyano/homebrew-tap installing the binary and the contrib adapters (ADR 0002).
* [Automate tagged releases](tasks/043-automate-releases.md) - Tag-driven Actions building macOS/Linux binaries into a GitHub Release.
* [Document installation in the README](tasks/044-document-installation.md) - Install section: brew install and cargo install --git, with the adapter caveat.
* [Add a demo recording to the README](tasks/045-add-repo-demo.md) - asciinema/GIF of the tree, search, and git badges.
* [Fill in Cargo package metadata](tasks/046-add-cargo-metadata.md) - repository / homepage / keywords / categories for discoverability.
* [Decide the crates.io publishing story](tasks/047-decide-crates-io-publish.md) - Publish as birch-tree (ADR 0002 fallback) vs Homebrew-only.
* [Add a changelog and issue templates](tasks/048-add-changelog.md) - CHANGELOG.md and optional GitHub templates.
* [Deduplicate and route the documentation set](tasks/049-dedup-and-route-docs.md) - Single home per topic across README / AGENTS / CONTRIBUTING / workflow.md / docs/; add docs/index.md; date the logs.

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

# Dropped
