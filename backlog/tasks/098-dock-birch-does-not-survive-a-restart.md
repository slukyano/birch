---
type: Task
title: The dock birch does not survive a cmux restart
description: A restored Dock snapshot brings back bare terminals, not controls, so birch is replaced by a shell on every cmux relaunch and nothing puts it back from the CLI.
status: Draft
priority: medium
---

Observed against `contrib/testcmux` on 2026-08-23, over two full quit-and-relaunch cycles with
session restore enabled:

- The Dock snapshot restores the **panes** and loses the **controls**. Each comes back as a plain
  interactive login shell titled `Terminal`; the `dock.json` command is not re-run.
- `dock.json` does not re-seed, because seeding runs only for a Dock with no saved snapshot.
- `cmux reload-config` does not help — it reloads Ghostty config and `cmux.json`, and the Dock was
  unchanged after it. No CLI verb reloads the Dock config.

So a dock birch's lifetime is one cmux session, and the way back is in-app only: reload the Dock
config from the app (which replaces the whole Dock), or type the command into the shell that
replaced the tree.

This is cmux's design rather than an adapter defect — cmux owns the Dock lifecycle, which is the
premise of [ADR 0016](../../docs/adr/0016-cmux-integrates-via-the-dock.md) — and it may be that
birch has nothing to build here. The task exists because "the tree is silently gone after an app
update" deserves a decision rather than an omission.

## Options, none yet chosen

- **Document it and stop.** [`086`](086-adr-0016-window-scoped-framing.md) already carries the
  sentence into `docs/integrations.md`. Cheapest, and honest, and leaves a person retyping a command
  after every cmux update.
- **Ask cmux for it.** Either re-running a restored control's command, or a CLI verb that reloads
  the Dock config, would close this entirely and both are cmux-side. Filing on another project's
  tracker needs maintainer approval per `AGENTS.md`, and the ask should carry the observation above
  rather than a feature request in the abstract.
- **A birch-side workaround.** There is no obvious one: birch is not running, so nothing of birch's
  is there to notice. Anything that watched for the Dock's return would be a resident process
  outside any host's lifecycle, which is the shape `docs/design.md` fences off.

## Verify before acting

The observation is from a throwaway instance with an ad-hoc-signed bundle and its own UserDefaults.
Before any upstream ask, confirm the same on a normal cmux — quit, relaunch, and look at whether the
Dock control came back — because a re-identified bundle is exactly the kind of thing session restore
could treat differently.
