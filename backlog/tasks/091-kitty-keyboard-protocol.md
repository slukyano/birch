---
type: Task
title: Adopt the Kitty keyboard protocol where the terminal supports it
description: The legacy encoding cannot represent Shift with a control character, so Ctrl-Shift-C is the same byte as Ctrl-C - which birch binds to quit.
status: Draft
priority: low
---

birch decodes keys with crossterm's **legacy encoding**: it never pushes
`PushKeyboardEnhancementFlags`, so what arrives is what a terminal has sent since the 1970s. In that
encoding `Ctrl` with a letter is transmitted as the control character — `Ctrl-C` is byte `0x03` —
and **`Shift` has no representation at all** alongside it.

The consequence found while designing [`028`](028-add-copy-paths.md): `Ctrl-Shift-C` and `Ctrl-C`
are indistinguishable, and `Ctrl-C` is quit ([ADR 0008](../../docs/adr/0008-q-types-into-search.md)).
So the `Ctrl-Shift-C` accelerator the design doc gave Copy Relative Path was not merely
undeliverable — binding it would have quit birch when a user reached for copy. It was dropped, and
the copy actions live on the menu.

The **Kitty keyboard protocol** encodes modifiers explicitly, including `Shift` with a control
character, and ghostty supports it. Adopting it would make `Ctrl-Shift-C` and its neighbours real
bindings — which matters more than one accelerator, because printable characters are permanently
reserved for search, and `Ctrl`-modified keys are most of the binding space birch has left.

## What a design has to answer

- **It changes how every key is decoded, not one.** Key *release* events start arriving (birch
  currently filters them by `KeyEventKind`), modifiers appear on keys that never carried them, and
  `Esc` stops being ambiguous — which is a benefit, but a behavioural change to the layered back-out
  ([ADR 0012](../../docs/adr/0012-esc-backs-out.md)).
- **Support is per terminal**, detectable with `crossterm::terminal::supports_keyboard_enhancement`.
  A binding that exists in ghostty and not in Terminal.app is a documentation problem at least, and
  possibly a reason to bind nothing that depends on it.
- **The flags have to be popped on every exit path** — including the panic hook and the child
  handover in `term::reenter`, which already carry mouse-capture state.
- **Which flags.** The protocol is a bitset; `DISAMBIGUATE_ESCAPE_CODES` alone buys most of the
  value at the least behavioural cost, while `REPORT_EVENT_TYPES` is what brings key releases.

## Related

- `028-add-copy-paths` — dropped the `Ctrl-Shift-C` accelerator over this.
- `073-hotkey-reference` — would list whatever this makes bindable.
