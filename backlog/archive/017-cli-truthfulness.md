---
type: Task
title: CLI truth — --open-cmd help, --no-socket
status: Done
priority: medium
---

Maintainer feedback: the `--open-cmd` help still claims `$EDITOR` is the default
(it is `$VISUAL` then `$EDITOR` since sprint 005), and the control socket binds by
default with no way to opt out.

## Design

- Fix the `--open-cmd` help string; mention `{line}` is reserved for future
  line-targeted opens (content search) so templates stay stable.
- Add `--no-socket`: skip binding entirely (no instance socket, no by-root symlink);
  `birch-ctl` then reports no instance. The default stays socket-on — the design doc's
  integration story depends on `birch-ctl reveal` working against a plain `birch`.
