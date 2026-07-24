---
type: Task
title: Open defaults prefer VISUAL over EDITOR
description: Default open command resolves $VISUAL, then $EDITOR, then the platform opener.
status: Done
priority: low
---

Maintainer feedback on the MVP: Enter opening `$EDITOR` was intentional, but the
conventional precedence for a full-screen context is `$VISUAL` first (`$EDITOR`
historically names the line editor). `--open-cmd` still overrides everything.

## Design

`OpenCmd::default_cmd` resolves `$VISUAL`, then `$EDITOR`, then `open`/`xdg-open`; the
resolution order is factored into a pure function over optional values so it is testable
without touching the process environment. README and the design doc's Opening-files
section say "$VISUAL / $EDITOR" where they said "$EDITOR".
