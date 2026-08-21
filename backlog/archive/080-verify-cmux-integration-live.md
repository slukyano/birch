---
type: Task
title: Verify the cmux integration live
description: Drive a separate cmux instance and confirm the three things the integration promises — sidebar, follow, preview — actually happen.
status: Done
priority: medium
---

The cmux integration was built across
[`018`](018-add-cmux-integration.md), [`021`](021-refine-cmux-adapter.md) and
[`024`](024-adopt-cmux-dock-integration.md), and each was verified as it landed. What has
never been checked is the whole thing at once, from an *installed* setup, against the three
behaviours the maintainer actually wants:

1. **birch is in the sidebar** — the Dock control starts, and one dock birch exists per window.
2. **It follows the current workspace** — switching workspace re-roots the tree, including after a
   session restore and after a directory rename.
3. **A preview opens in the current workspace** — as a new tab, in the workspace that is selected,
   which is [`079`](079-preview-opens-in-current-workspace.md)'s subject.

Plus the two verbs a person invokes directly: `dock-socket` resolving the same path from the dock
birch and from a pane in that window, and reverse-reveal through it.

## How it must be run

**Never against the maintainer's own cmux instance** (`AGENTS.md`) — always a throwaway one. The
recipe `AGENTS.md` carried (`open -na cmux`) does not produce a second instance; what does is in
the design below, and `AGENTS.md` is corrected as part of this task.

## Done when

Each of the three behaviours is observed automatically and recorded. Anything that fails becomes a
fix in this sprint or a `Draft` task with a name.

## Design

**All of it is automatable.** The premise this task was filed on — that the Dock is a UI surface a
script cannot reach — is wrong twice over: a separate cmux instance *can* be run, and the Dock is
fully scriptable over the control socket.

### Running a second cmux

`AGENTS.md`'s recipe (`open -na cmux`) does not work: cmux enforces one instance **per bundle
identifier**, and a second process exits right after `appDelegate.didFinish.complete` without a
crash report. `CMUX_TAG`, which does give a separate socket and state, is gated to DEV builds
(*"refusing to launch untagged cmux DEV"*).

What works is a **copy of the bundle carrying its own identifier**:

```sh
cp -a /Applications/cmux.app ./cmux-birchtest.app
plutil -replace CFBundleIdentifier -string com.cmuxterm.app.birchtest \
    cmux-birchtest.app/Contents/Info.plist
codesign --force --deep --sign - --options runtime --entitlements ent-adhoc.plist cmux-birchtest.app
open -na "$PWD/cmux-birchtest.app" --env CMUX_SOCKET_ENABLE=1 --env CMUX_SOCKET_MODE=allowAll \
    --env "CMUX_SOCKET_PATH=$SOCK" --env CMUX_ALLOW_SOCKET_OVERRIDE=1 \
    --env CMUX_DISABLE_SESSION_RESTORE=1
```

Ad-hoc signing cannot claim the team identifier or the keychain group, so those two entitlements are
dropped and the rest kept. `open` needs an **absolute** path — a relative one is "not found".

A distinct identifier isolates more than the instance check: **UserDefaults are per bundle id**, so
the copy has its own settings and its own session, and nothing the test does can reach the
maintainer's cmux. The Dock beta is off in a fresh instance and is enabled the same way:
`defaults write com.cmuxterm.app.birchtest rightSidebar.beta.dock.enabled -bool true`.

### Driving the Dock

No `dock.json` is needed, which also side-steps the project-config **trust gate**:

| Step | Command |
|---|---|
| Create the Dock surface | `cmux new-surface --placement dock` → `[dock:global]` in `cmux tree` |
| Run the adapter in it | `cmux send --surface <dock> "…/birch-cmux dock-run\n"` |
| **Read birch's screen** | `cmux capture-pane --surface <dock>` |
| Drive birch | `cmux send-key --surface <dock> down` / `enter` |
| See where a preview landed | `cmux tree --json` → the non-terminal surface's workspace |

`capture-pane` on a Dock surface is what makes the tree itself observable — the root line and rows
come back as text.

### The checks

Each is an assertion, not an eyeball:

1. **Sidebar** — after `dock-run`, `capture-pane` shows birch's root row.
2. **Follows the workspace** — `workspace select` each way; the captured root line changes to that
   workspace's directory.
3. **Preview lands in the current workspace** — record `selected_workspace_ref`, press `enter` in the
   dock birch, then assert the new `filepreview` surface's workspace **is** that one.

Also worth pinning, since it is what makes the whole thing work: the dock shell exports
`CMUX_SURFACE_ID` for the dock surface and `CMUX_WORKSPACE_ID` set to the **window** uuid (a routing
id cmux itself rejects), which is precisely why an `open` without `--window` lands in the dock.

### Already run, against the unfixed adapter

The harness was built and exercised during design, so the task starts from evidence rather than
hope. With `AlphaWs` selected and birch live in the Dock, `enter` on `alpha-file.txt` put the
preview in `workspace:1` — a different workspace — which `cmux open` then selected. Re-rooting
passed in both directions. And the fix [`079`](079-preview-opens-in-current-workspace.md) proposes
was confirmed the same way: with `BetaWs` selected, `cmux open FILE --window window:1` from a
dock-shell environment landed in `BetaWs`.

### Where the harness lives

`$CLAUDE_JOB_DIR` is disposable, so the driver script becomes a committed artifact — a `testcmux`
helper plus the check script — with the bundle copy built on demand and never checked in. It does
**not** join `cargo test`: it needs a GUI session and a 314 MB bundle copy, which a headless CI run
cannot provide. It is a script the maintainer or an agent runs deliberately.

## Result

Nine assertions, all passing, in [`contrib/tests/cmux-integration.sh`](../../contrib/tests/cmux-integration.sh):

| | Check |
|---|---|
| 1 | birch runs in the Dock, rooted at the selected workspace |
| 2 | selecting `BetaWs` re-roots the tree |
| 3 | selecting `AlphaWs` re-roots it back |
| 4–5 | a preview from the dock birch opens in the selected workspace — checked from **both** workspaces |
| 6 | `dock-socket` names an existing socket |
| 7 | `dock-socket` addresses the *running* dock birch (`birch ctl … get-path` answers on it) |
| 8 | a nested file starts hidden |
| 9 | `reveal` through `dock-socket` shows it |

The suite runs against the built `birch-cmux` **binary** rather than the script, so it also
exercises [ADR 0027](../../docs/adr/0027-adapters-ship-as-binaries.md)'s `$0`: `dock-run` bakes it
into birch's `--open-cmd`, and `preview` only arrives if it is a path that invokes the adapter again.

**It discriminates.** Counter-checked against the adapter as it stood before
[`079`](079-preview-opens-in-current-workspace.md) (`82635d0`): exactly the two preview assertions
fail — previews land in `workspace:1` whatever is selected — and the other seven pass.

Two harness bugs were found and fixed while writing it, both worth remembering:

- **Cleanup by closing preview surfaces does not work** — there is no CLI verb that closes one
  (`cmux close` is not a command). The check now diffs the set of preview surfaces across the open
  and looks at what is *new*, which is more direct anyway.
- **`down` + `enter` is only "open that file" if the workspace has one row.** With a folder sorted
  first, `enter` expanded it and nothing opened. The reveal check's folder is therefore created
  after the preview rounds.

And one real hazard, hit for real: with `CMUX_SOCKET_PATH` set to an empty string the CLI falls back
to the default socket — i.e. to the maintainer's own cmux. Two stray workspaces were created there
and immediately removed. `testcmux` now passes `--socket` explicitly on every call and refuses any
path outside its own working directory.
