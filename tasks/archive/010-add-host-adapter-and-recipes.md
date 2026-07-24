---
type: Task
title: Ship the reference host adapter and recipes
description: birch-herdr reference adapter plus nvim/tmux forward- and reverse-integration recipes.
status: Done
priority: medium
blocked_by:
- 009-add-control-socket
---

Phase 0.4 (partial) of [the design doc](../docs/design.md): one reference adapter
(birch-herdr) documenting the pattern — spawn with `--socket` + `--open-cmd`, toggle binding,
reverse-reveal via `birch-ctl reveal` — plus plain recipes for tmux and editors without
adapter hooks. Adapters live with the host; the repo ships the reference and the recipes.
Verify SGR mouse passthrough in herdr early.

## Design

`contrib/` ships two reference adapter scripts plus `docs/integrations.md`:

- **`contrib/birch-herdr`** — the flagship adapter, written against herdr's CLI surface
  (`herdr pane split --direction right --ratio … --no-focus`, `pane run`, `pane
  send-text`, `pane close`): spawns the side pane running
  `birch --socket <adapter-chosen path> --open-cmd 'birch-herdr open-in-main {}'`,
  implements `open-in-main` by sending the open command to the originating pane, and a
  `toggle` subcommand that closes/respawns the pane. Reverse-reveal is one line the
  editor calls: `birch-ctl --socket <path> reveal <file>`.
- **`contrib/birch-tmux`** — the same pattern over tmux (`split-window -h -l 30%`,
  `send-keys`, `kill-pane`), end-to-end testable headless (`tmux new-session -d`), which
  makes it the adapter the test suite exercises.
- **`docs/integrations.md`** — the adapter pattern (birch's whole promise: socket
  protocol, `--socket`, `--open-cmd`, clean SIGHUP exit), both scripts explained, plus
  plain recipes: forward integration without any adapter (`--open-cmd` templates for
  `nvim --server`, `emacsclient`, `code -g`), reverse integration (a two-line editor
  autocmd calling `birch-ctl reveal`), tmux mouse-mode FAQ, and the single-instance
  discipline note (the host's job — documented, not enforced).

Verification: the tmux adapter runs in a headless tmux session in tests/CI fashion;
birch-herdr is verified against the herdr CLI surface only — live verification (including
the design doc's SGR mouse passthrough check) needs an interactive herdr session and
becomes the follow-up task `verify-herdr-integration`.
