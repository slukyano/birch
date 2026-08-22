---
type: Task
title: Add copy name and paths
description: Copy Name / Relative Path / Absolute Path over OSC 52 with fallbacks - split out of the 0.5 bundle.
status: Designed
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

### The clipboard: one transport, chosen the way Neovim chooses it

`docs/design.md` lists the mechanics as a fallback order — OSC 52, then a native tool, then the
status line. **That order cannot be implemented as written**, because OSC 52 is a one-way write:
there is no reply, no acknowledgement, and no error, so nothing can detect that it failed and reach
for `pbcopy` afterwards. A reactive chain needs a failure to react to, and there is none.

Surveying what comparable tools do produced three shapes, not one:

| tool | mechanism |
|---|---|
| **yazi** | writes OSC 52 **and** shells out on every copy, first success wins, no detection |
| **Helix** | one provider from a detected chain (tmux → `pbcopy`/Termux → `wl-copy` → `xclip` → `xsel`), with OSC 52 last and behind a compile-time feature |
| **Neovim** | one provider; normal tool discovery, **except** OSC 52 when `$SSH_TTY` is set |
| **nnn** | shells out to a helper script that does its own platform detection; overridable by `$NNN_COPIER` |
| **broot** | an in-process crate behind a compile-time feature — and the Homebrew build ships it *off*, so `copy_path` reports "Clipboard feature not enabled at compilation" |

**birch follows Neovim: exactly one transport, and `$SSH_TTY` decides.**

1. **`$SSH_TTY` is set** → **OSC 52 only**. birch is running on a remote machine, and a native tool
   there would write a clipboard nobody will ever paste from. OSC 52 travels back down the same tty
   to the terminal the user is actually sitting at.
2. **Otherwise** → **the first native tool found**: `pbcopy` on macOS, `wl-copy` when
   `$WAYLAND_DISPLAY` is set, `xclip -selection clipboard` when `$DISPLAY` is, then `xsel`. This
   matters more than a backstop — Apple Terminal does not support OSC 52 at all, so on the stock
   macOS terminal it is the only thing that works.
3. **No native tool found** → **OSC 52**, because nothing else is available. Its silence costs
   nothing here: there is no alternative to fall back to.

No transport is ever written twice, so a clipboard manager sees one event per copy and no process
fights another for X11 selection ownership. **`tmux load-buffer` is not used** — locally the native
tool already works inside tmux, and remotely `$SSH_TTY` has already routed to OSC 52, which tmux
forwards with `set-clipboard on`.

**Shelling out, not an in-process crate.** `arboard` and its neighbours link the X11/Wayland client
stack into what is otherwise a small terminal binary released for three targets, and on X11 the
copying *process owns the selection* — quit birch and the clipboard is empty, because nothing is
left to serve it. `xclip` solves that by forking and outliving its parent, which is exactly what an
in-process implementation cannot do. broot is the cautionary case: the feature exists in the source
and is compiled out of the build people install.

**The detection is load-bearing, and that is the accepted cost.** If `$SSH_TTY` is unset while the
session is genuinely remote — mosh is the case to worry about — birch writes the wrong machine's
clipboard and says nothing. The remedy if anyone hits it is a config key naming the transport, which
is what both Neovim and Helix expose; it is not built now, because nobody has asked for it.

**Implementation details that are traps rather than choices:**

- **`xclip` defaults to the PRIMARY selection.** It needs `-selection clipboard`, or it writes the
  middle-click buffer instead of the one `⌘V` reads.
- **The child's stdout and stderr go to `/dev/null`.** birch is in the alternate screen; `xclip`
  complaining about a missing `$DISPLAY` would scribble across the tree.

**The status line confirms what was copied** — *"Copied 3 paths"* — rather than serving as a third
tier. In the OSC 52 case nothing can know whether it landed, so the message says what birch did, not
what the clipboard now holds.

**This rewrites `docs/design.md`'s "Clipboard mechanics" list**, which becomes one chosen transport
and a confirmation rather than three tiers tried in order.

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
- **`crates/birch-core/src/clipboard.rs`** — the transport choice, the OSC 52 encoder, and the
  native-tool spawn. No ratatui, and no terminal handle: the choice is a pure function of the
  environment and the OSC 52 payload is *built* here and *written* by the caller, so both stay
  unit-testable without a terminal or a clipboard.
- **`crates/birch/src/app.rs`** — `perform` arms for the three actions; `format_path` is reused
  unchanged, joined with newlines over `view.selected(&rows)`.
- **`docs/design.md`** — the Clipboard mechanics list rewritten; the `Ctrl-Shift-C` row removed from
  the Keyboard table and from the menu sketch.

### Tests and verification

Unit: the transport choice for each environment — `$SSH_TTY` set, unset with a tool present, unset
with none — driven through an injected environment rather than `std::env`, so the cases are
reachable without mutating a process-global; the OSC 52 encoding, including a payload with non-ASCII
and one with a newline in a file name; the newline join in tree order for a multi-selection; each
`PathForm` resolving as `get-path` does for the same row, including the root's `.`; availability
with nothing selectable. Hands-on: drive the binary, copy from the menu, and read the clipboard back
with `pbpaste` — the one check that proves the transport rather than the encoder.
