---
type: Task
title: Add copy name and paths
description: Copy Name / Relative Path / Absolute Path over OSC 52 with fallbacks - split out of the 0.5 bundle.
status: Draft
priority: medium
---

Split out of `add-file-operations` so it can ship earlier (maintainer asked about
copying during MVP feedback): the copy primitive (name / root-relative / absolute) with
the OSC 52-first clipboard chain (works over SSH and in tmux), native fallback
(`pbcopy`/`wl-copy`/`xclip`), status-line last resort. `Ctrl-Shift-C` copies the
relative path (the design doc's accelerator); the other variants arrive with the
context menu. Terminal Cmd+C stays what it is — the terminal's own selection copy
(Shift-drag under mouse capture).

## Design

The primitive already exists. `docs/design.md` calls it out — *"resolve selected node →
name/relpath/abspath is one primitive with three transports: clipboard (menu), stdout (`--pick`),
socket (`get-path`)"* — and sprint 020 built it as `App::format_path(path, form)` for the socket
transport. This task adds the third transport and the three actions that reach it.

### Three actions, one resolver

`birch_core::action` gains `CopyName`, `CopyRelativePath`, and `CopyAbsolutePath`, filling the copy
group `MENU_GROUPS` already reserves. Each maps to the `PathForm` the socket already speaks, so
there is one definition of what "the relative path" means and the menu and `birch ctl get-path`
cannot drift apart.

**With several rows selected, a copy joins them with newlines**, in tree order — the shape
`074-add-multi-selection` named as the obvious first consumer. **The labels do not change with the
count.** A menu whose entries rewrite themselves as the selection changes is noisier than one that
does not, and the count belongs in the confirmation rather than in the label.

### The clipboard: both transports, never a reactive fallback

`docs/design.md` lists the mechanics as a fallback order — OSC 52, then a native tool, then the
status line. **That order cannot be implemented as written, because OSC 52 gives no answer.** It is
a write to the terminal; there is no reply, no acknowledgement, and no error. Nothing can detect
that it failed and reach for `pbcopy` afterwards.

So the design stops pretending the chain is reactive:

1. **OSC 52 is always written.** `ESC ] 52 ; c ; <base64> BEL`. It is the only transport that works
   over SSH or inside tmux with `set-clipboard on`, and it is native in ghostty — the flagship
   environment, and exactly where a subprocess clipboard fails silently.
2. **A native tool is also run when one is on `PATH`** — `pbcopy`, else `wl-copy`, else `xclip`.
   Not *instead*: both. Locally the two set the same clipboard to the same bytes, so the second
   write is harmless; over SSH the native tool sets a remote clipboard nobody looks at while OSC 52
   sets the real one. Spawning it costs a process and needs no environment sniffing, which is the
   point — detection would have to guess at SSH, tmux, screen, and nested combinations of all three.
3. **The status line confirms what was copied** — *"Copied 3 paths"* — rather than serving as a last
   resort. A last resort implies something was known to have failed, and nothing here can know that.

**This narrows `docs/design.md`'s "Clipboard mechanics" list**, which is rewritten to describe two
transports written together and a confirmation, not three tiers tried in order.

### `Ctrl-Shift-C` is not bindable, and binding it would be dangerous

The design doc gives Copy Relative Path the accelerator `Ctrl-Shift-C`. In the **legacy key
encoding** — the one birch uses, since it never pushes crossterm's keyboard-enhancement flags —
`Ctrl` with a letter is transmitted as the control character (`Ctrl-C` → `0x03`), and **`Shift` is
not representable at all**. `Ctrl-Shift-C` and `Ctrl-C` are therefore the same byte, and `Ctrl-C` is
**quit** ([ADR 0008](../../docs/adr/0008-q-types-into-search.md)).

Binding it would not merely fail to work: reaching for copy would quit birch. So **the accelerator is
dropped**, the three copy actions are menu-only, and `docs/design.md`'s keyboard table loses its
`Ctrl-Shift-C` row.

Making it work needs the **Kitty keyboard protocol**, which does encode `Shift` with a control
character and which ghostty supports. Pushing those flags changes how *every* key is decoded — key
release events start arriving, modifiers appear on keys that never carried them — so it is a change
to birch's whole input layer, not an accelerator, and is filed as its own task rather than smuggled
in here.

### Public-surface delta

| surface | change |
|---|---|
| key bindings | **none** — and `Ctrl-Shift-C` is removed from the documented table, never having worked |
| menu | three new entries: Copy Name, Copy Relative Path, Copy Absolute Path |
| CLI flags · config keys · environment variables | none |
| socket / protocol | none |
| on-disk paths / formats | none |
| library API | `birch_core::action` gains three variants |

### Implementation

- **`crates/birch-core/src/action.rs`** — the three variants, their labels, and availability (any
  live selection; `CopyName` and the two paths differ only in the `PathForm` they resolve).
- **`crates/birch-core/src/clipboard.rs`** — the OSC 52 encoder and the native-tool spawn. No
  ratatui, and no terminal handle: the OSC 52 payload is *built* here and *written* by the caller,
  so the encoder is a pure function over a string and stays unit-testable.
- **`crates/birch/src/app.rs`** — `perform` arms for the three actions; `format_path` is reused
  unchanged, joined with newlines over `view.selected(&rows)`.
- **`docs/design.md`** — the Clipboard mechanics list rewritten; the `Ctrl-Shift-C` row removed from
  the Keyboard table and from the menu sketch.

### Tests and verification

Unit: the OSC 52 encoding, including a payload with non-ASCII and one with a newline in a file name;
the newline join in tree order for a multi-selection; each `PathForm` resolving as `get-path` does
for the same row, including the root's `.`; availability with nothing selectable. Hands-on: drive the
binary, copy from the menu, and read the clipboard back with `pbpaste` — the one check that proves
the transport rather than the encoder.
