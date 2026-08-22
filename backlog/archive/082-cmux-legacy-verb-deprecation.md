---
type: Task
title: birch-cmux calls a deprecated cmux verb
description: The adapter uses `cmux list-workspaces`, which now prints a deprecation notice; the canonical form is `cmux workspace list`.
status: Dropped
priority: low
---

`window_selected_dir()` in [`contrib/birch-cmux`](../../contrib/birch-cmux) calls:

```sh
cmux list-workspaces --window "$1" --json
```

cmux now answers that with:

> `cmux: 'list-workspaces' is now an alias for 'cmux workspace list'. The legacy form keeps working
> indefinitely; set CMUX_QUIET=1 to silence this notice.`

## Not a defect today

Checked while the notice was reported:

- The notice goes to **stderr**, and the call already redirects `2>/dev/null`, so nothing reaches
  the user through birch.
- **stdout stays clean JSON** — it parses, so the workspace lookup is unaffected. Had the notice
  gone to stdout, `window_selected_dir` would have failed its `json.loads` and returned nothing,
  and the follow watcher would have stopped re-rooting without a word.
- `cmux workspace list --window <id> --json` accepts the same flags and prints no notice.

So this is hygiene, not breakage, and cmux promises the legacy form works indefinitely.

## Do

Move the call to `cmux workspace list`. Check the other adapters and the test harness at the same
time — a sweep at the time of writing found `list-workspaces` only in this one place, and the rest
of the calls (`current-window`, `tree`, `events`, `open`, `workspace create`, `workspace select`)
are already canonical.

Worth pairing with any other cmux-CLI drift found by then, rather than spending a commit alone.

## Dropped

Folded into [`083`](../tasks/083-birch-cmux-does-three-things.md), which rewrites the call site anyway. Two
tasks editing the same line is worse than one.
