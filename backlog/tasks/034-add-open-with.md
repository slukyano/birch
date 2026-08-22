---
type: Task
title: Add "Open with…" to the context menu
description: Choose an alternative open command for the selected node.
status: Draft
priority: low
blocked_by:
- 029-add-file-operations
---

Listed as "(later)" in the context-menu sketch of [the design doc](../../docs/design.md):
an "Open with…" entry offering alternative open commands. Needs a design for where the
alternatives come from (config?) before it can be scheduled; post-MVP.

## Design

The whole task is one product decision — where the alternatives come from — and everything else
follows from it cheaply, because birch already has an open primitive and a template contract.

### Where the alternatives come from: the config file

`birch.toml` ([ADR 0022](../../docs/adr/0022-config-file.md)) gains an `open_with` array. Each entry
is a label and a command template using **exactly the contract `--open-cmd` already defines** — `{}`
for the path, `{line}` where a line is known, and a `detached` flag for commands that are not tty
programs:

```toml
[[open_with]]
label = "Zed"
cmd = "zed {}"
detached = true

[[open_with]]
label = "Hex editor"
cmd = "hx {}"
```

**Not the operating system's handler list.** macOS LaunchServices and Linux `.desktop` handlers would
need no setup, which is their whole appeal, but they fail both halves of the identity test
([ADR 0028](../../docs/adr/0028-the-identity-is-the-scope-fence.md)). The explorer panel this
identity names lists *editors and viewers*, not every application registered for a MIME type — and a
list of thirty GUI applications does not survive a thirty-column pane. Querying them also means
shelling out to platform tools that are slow, differently named per platform, and absent on the
remote machines birch is often run on.

Reusing the `--open-cmd` contract means no second template language, no new escaping rules, and that
a user who has already written an open command knows how to write this one.

### No submenu: each entry is a menu entry

`docs/design.md` sketches a single `Open with…` row, whose ellipsis implies a second level. **This
design flattens it**: each configured entry becomes its own row in its own group — `Open in Zed`,
`Open in Hex editor` — and there is no nested menu.

The reason is proportion. A submenu is new machinery — its own geometry, its own keyboard traversal,
its own edge cases at the pane's edges — for a list the user curates by hand and which is therefore
short. Flat entries need none of it, and read better besides: the menu says what will happen instead
of promising a further choice. If a list ever grows past a handful, a submenu can be added later
**without changing the config surface**, which is the part that would be expensive to get wrong.

**With no `open_with` configured, nothing appears.** The action is unavailable and an unavailable
action is absent (ADR 0029), so the default menu is exactly what it is today. There is no empty
submenu and no greyed row advertising a feature the user has not set up.

**This narrows `docs/design.md`'s menu sketch**, whose `Open with… (later)` row is replaced by a note
that configured entries appear as their own rows.

### Availability

- **Exactly one target.** Opening several files in several applications is not this feature.
- **Unavailable in `--pick`**, where `Open` means *confirm* and handing the path to another program
  instead would defeat the mode.
- **Unavailable when `open_with` is empty**, which is the default.

### Public-surface delta

| surface | change |
|---|---|
| config key | **`open_with`** — an array of `{ label, cmd, detached }` tables, `cmd` using the `--open-cmd` template contract |
| menu | one row per configured entry, in its own group |
| CLI flags · environment variables | none |
| socket / protocol | none |
| on-disk paths / formats | none beyond the config key |
| library API | `birch_core::action` gains one variant carrying the entry's index |

### Implementation

- **`crates/birch-core/src/config.rs`** — the `open_with` array, parsed tolerantly like every other
  key: a malformed entry is dropped rather than failing the load.
- **`crates/birch-core/src/action.rs`** — `Action::OpenWith(usize)`, indexing the configured list;
  its label comes from the entry rather than from a `&'static str`, which is the first action whose
  label is not a constant.
- **`crates/birch/src/app.rs`** — a `perform` arm building an `OpenCmd` from the entry and reusing
  `perform_open` unchanged, so tty handover and detached spawning behave exactly as `--open-cmd`
  does.

### Tests and verification

Unit: config parsing including a malformed entry being dropped; availability with an empty list, with
several targets, and in `--pick`; the label coming from the entry; the template resolving `{}` and
`{line}` identically to `--open-cmd`. Hands-on: configure two entries, open each from the menu, and
confirm one hands over the terminal while the `detached` one does not.
