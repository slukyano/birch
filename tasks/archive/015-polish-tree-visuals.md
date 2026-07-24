---
type: Task
title: Polish tree visuals — LICENSE icon, IDEA-style match boxes, root path
status: Done
priority: high
---

Maintainer feedback with an IDEA screenshot as the reference.

## Design

- **LICENSE icon**: `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING`, `NOTICE` get a
  law-scales glyph via the by-name table (they have no extension to match).
- **Match highlighting**: matched characters render as IDEA-style boxes — accent
  background with dark foreground — instead of accent-colored text; unmatched text keeps
  its style. The match color constant becomes a bg/fg pair.
- **Root path annotation**: the root row shows the directory name plus the full path,
  dimmed, `$HOME` abbreviated to `~` (IDEA-style). `Row` gains an `annotation` rendered
  after the label; only the root sets it. The bottom line drops the now-duplicate root
  path and shows it only as message context.
