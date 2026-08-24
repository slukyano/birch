---
type: Task
title: birch-tmux and birch-herdr still name themselves in the Open row
description: The display name added by 096 is set only by birch-cmux; the other two adapters keep showing "Open with birch-tmux" and "Open with birch-herdr".
status: Draft
priority: low
blocked_by:
- 096-open-with-names-the-adapter
---

[`096`](../archive/096-open-with-names-the-adapter.md) gives the open command a display name and sets it in
`birch-cmux`, so the menu row reads `Open with cmux`. The other two adapters bake themselves into
`--open-cmd` the same way — `contrib/birch-tmux:65` and `contrib/birch-herdr:57` — and keep the
label they had:

```
Open with birch-tmux
Open with birch-herdr
```

Neither is a program a person opens files with. It is the same defect `096` describes, on the two
hosts that sprint did not run.

## Do

Set the display name in both scripts, through whichever surface `096`'s design settles on. Expected
values `tmux` and `herdr`. One line per script.

## Why it is its own task

`096` ships in [`sprint-022`](../sprints/sprint-022.md), which touches no host but cmux and has no
way to run tmux or herdr. Editing an adapter that the sprint cannot exercise was rejected in favour
of this split, so the label change travels with [`sprint-023`](../sprints/sprint-023.md), where a
live session observes it.

The split is what keeps `096` honest: it closes when the mechanism exists and the host it was
reported against is fixed, rather than closing over two instances nobody looked at.

## Verify

The row is drawn, so it is read rather than asserted: open the menu in a birch spawned by each
adapter and look at the `Open` row. In herdr that is part of the live session
[`035`](035-verify-herdr-integration.md) runs; in tmux it is a pane and a right-click.
