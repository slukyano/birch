---
type: ADR
title: The open command is settable at runtime, as its own socket verb
status: Proposed
sprint: sprint-022
---

# Context

`--open-cmd` is launch-only. A host adapter that needs the command to name something it cannot know
until launch has one option: compute it in the adapter and bake it in. `birch-cmux` did neither
cleanly — it baked a callback into *itself* (`birch-cmux preview {}`), and the callback re-resolved
the window on every open, from focus, at a moment when focus may be anywhere.

Removing that callback ([`083`](../../backlog/tasks/083-birch-cmux-does-three-things.md)) means the
adapter sets the real command — `cmux open {} --window <uuid>`. The window is known at launch, from
the dock control's own environment, so baking alone would work. It is not enough on its own: a dock
birch moved between windows would keep a command naming the window it was born in, and nothing
would correct it. The launch root has exactly this shape already, and `set-root` is how it is
corrected.

A second caller arrives with it.
[`096`](../../backlog/tasks/096-open-with-names-the-adapter.md) gives the open command a display
name, so the menu stops reading `Open with birch-cmux`. A command that can be replaced at runtime
but cannot carry its own name would reintroduce the wrong label through the new surface.

# Decision

- **A new socket verb, `set-open-cmd`**, and its client form
  `birch ctl set-open-cmd <template> [--name <text>] [--detached|--no-detached]`. It replaces the
  running birch's open command, its display name, and its mode. Absent optional parts leave the
  corresponding part unchanged.
- **Its own verb, not a `SettingKey` under `set`.** `set` is the display-settings verb and is today
  incapable of making birch execute anything; folding an executable command into it would change
  what the verb means with no signal to a reader that one key is unlike the rest. `set` also feeds
  persistence, which this must stay out of.
- **The mode travels with the command.** A template swapped without its `Terminal`/`Detached` mode
  would let an adapter hand a Dock pane's terminal to a GUI dispatcher and corrupt the screen.
- **Nothing about the open command is persisted.** A host-specific command must not outlive the
  host, or a birch launched later under a different host inherits a command for a host that is not
  running.
- **Adapters still bake a correct command at launch.** The setter corrects; it does not initialise.
  There is no "unset" state and no interval in which the command is a `VISUAL`/`EDITOR` fallback.
- **A launch flag `--open-name <text>`** for the display name, so adapters that bake and never set —
  `birch-tmux`, `birch-herdr` — can name themselves too.

# Consequences

- **A socket peer can choose a command birch will execute.** The socket is `0600` inside a `0700`
  directory, so that peer is the same user, who can already run anything birch could run. No
  privilege boundary is crossed and no mechanism is added. It is stated here because "a client can
  make birch exec this" belongs in a document rather than in an inference from a file mode.
- **The display name is attacker-free but not shape-free.** It is drawn in a menu row, so it is
  bounded and stripped of control characters at the boundary, like any other text arriving over the
  socket.
- **Additive, per ADR 0011.** A new verb and two new optional request fields; no existing field is
  redefined, and a client that has never heard of `set-open-cmd` is unaffected.
- **birch's promise to adapters is unchanged in kind.** The promise is the socket protocol,
  `--socket`, `--open-cmd` with `--open-detached`, and a clean SIGHUP exit. This adds a verb to the
  first of those, which is the part designed to grow, rather than a fifth promise.

# Alternatives considered

- **Bake only, no setter.** Simpler, no new surface, and correct until a dock birch is moved between
  windows — at which point every open goes to the wrong window with nothing to fix it. Rejected by
  the maintainer when `083` was framed.
- **`Verb::Set` with a `SettingKey::OpenCmd`.** The smallest diff. Rejected above: it changes what
  `set` means, and it drags the open command into persistence.
- **A `--follow-cmd` or a host mode inside birch.** Rejected on sight — that is the plugin instinct
  `docs/design.md` fences off. birch executes a command a client hands it; it knows nothing about
  cmux.
