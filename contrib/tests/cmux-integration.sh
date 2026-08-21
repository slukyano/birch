#!/bin/sh
# cmux-integration.sh — assert what the cmux integration promises, against a
# throwaway cmux (contrib/testcmux). Never touches a real instance.
#
#   contrib/tests/cmux-integration.sh [path/to/birch]
#
# Checks, in the maintainer's own words:
#   1. there is a birch in the sidebar
#   2. it always switches to the current workspace's cwd
#   3. it always opens previews in a new tab in the current workspace
#
# Not part of `cargo test`: it needs a GUI session and a copy of the cmux
# bundle, neither of which a headless CI run has.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
contrib=$(dirname "$here")
repo=$(dirname "$contrib")
testcmux="$contrib/testcmux"
birch="${1:-$repo/target/debug/birch}"

[ -x "$birch" ] || { echo "no birch at $birch (cargo build first)" >&2; exit 1; }

# Prefer the built `birch-cmux` binary — that is what an install puts on PATH,
# and running it also proves the shim hands the script a usable `$0`, since
# `dock-run` bakes `$0` into birch's --open-cmd and `preview` arrives through it.
adapter="$(dirname "$birch")/birch-cmux"
[ -x "$adapter" ] || adapter="$contrib/birch-cmux"
echo "adapter: $adapter"

work="${TESTCMUX_HOME:-${TMPDIR:-/tmp}/birch-testcmux}/work"
# Exactly one entry per workspace, so `down` then `enter` in the dock birch is
# unambiguously "open that file". The nested folder for the reveal check is
# created later, once the preview rounds are done.
rm -rf "$work"; mkdir -p "$work/ws-alpha" "$work/ws-beta"
: > "$work/ws-alpha/alpha-file.txt"
: > "$work/ws-beta/beta-file.txt"

cli() { "$testcmux" cli "$@"; }
pass=0; fail=0
check() { # check <label> <expected> <actual>
    if [ "$2" = "$3" ]; then
        pass=$((pass + 1)); printf 'ok    %s\n' "$1"
    else
        fail=$((fail + 1)); printf 'FAIL  %s\n        want: %s\n        got:  %s\n' "$1" "$2" "$3"
    fi
}

# Same, but stops the run. Used where continuing would be unsafe rather than
# merely uninformative: if birch is not rooted where we expect, we cannot assume
# the adapter is talking to the throwaway, and everything after this opens files.
check_fatal() {
    check "$@"
    [ "$2" = "$3" ] || { echo "        (fatal — not continuing)" >&2; exit 1; }
}

# The window's selected workspace, as a ref.
selected() { cli tree --json 2>/dev/null | python3 -c \
    'import json,sys; print(json.load(sys.stdin)["windows"][0]["selected_workspace_ref"])'; }

# Every preview surface, as "<surface ref> <workspace ref>" lines. The check
# below diffs this across the open rather than closing surfaces between rounds:
# there is no CLI verb that closes one, and a stale surface would otherwise be
# mistaken for the new one.
previews() { cli tree --json 2>/dev/null | python3 -c '
import json, sys
for w in json.load(sys.stdin)["windows"]:
    for ws in w["workspaces"]:
        for p in ws["panes"]:
            for s in p["surfaces"]:
                if s["type"] != "terminal":
                    print(s["ref"], ws["ref"])'; }

settle() { perl -e 'select(undef, undef, undef, '"${1:-2}"')'; }

[ -d "${TESTCMUX_HOME:-${TMPDIR:-/tmp}/birch-testcmux}/cmux-testcmux.app" ] || "$testcmux" build
"$testcmux" stop; settle 2
# Under `set -eu` any abort would otherwise leave the throwaway running.
trap '"$testcmux" stop' EXIT INT TERM
"$testcmux" start >/dev/null

alpha=$(cli workspace create --name AlphaWs --cwd "$work/ws-alpha" 2>/dev/null | awk '{print $2}')
beta=$(cli workspace create --name BetaWs  --cwd "$work/ws-beta"  2>/dev/null | awk '{print $2}')
settle 2

# A Dock surface, then the adapter inside it — the same entrypoint dock.json calls.
dock=$(cli new-surface --placement dock --focus false 2>/dev/null | awk '{print $2}')
settle 2
cli workspace select "$alpha" >/dev/null
settle 1
# The adapter runs inside the dock shell and calls bare `cmux` — for
# current-window, list-workspaces, events, and (through birch's --open-cmd) the
# *mutating* `cmux open`. The throwaway does propagate CMUX_SOCKET_PATH into the
# shells it spawns, but nothing here guarantees that, and an empty value sends
# the CLI to the default socket — a real cmux. So set it explicitly, and quote
# the paths so a directory with a space does not break the command line.
cli send --surface "$dock" \
    "clear; CMUX_SOCKET_PATH='$("$testcmux" sock)' BIRCH='$birch' '$adapter' dock-run\n" >/dev/null
settle 5

# 1. birch is in the sidebar, rooted at the selected workspace.
root_line() { cli capture-pane --surface "$dock" 2>/dev/null | sed -n '1p'; }
case "$(root_line)" in
*ws-alpha*) check_fatal "birch runs in the Dock, rooted at the selected workspace" yes yes ;;
*)          check_fatal "birch runs in the Dock, rooted at the selected workspace" yes "no (root line: $(root_line))" ;;
esac

# 2. it follows the current workspace, both ways.
cli workspace select "$beta" >/dev/null; settle 3
case "$(root_line)" in *ws-beta*) got=yes ;; *) got="no ($(root_line))" ;; esac
check "selecting BetaWs re-roots the tree" yes "$got"

cli workspace select "$alpha" >/dev/null; settle 3
case "$(root_line)" in *ws-alpha*) got=yes ;; *) got="no ($(root_line))" ;; esac
check "selecting AlphaWs re-roots it back" yes "$got"

# 3. a preview opens in the workspace you are in. Driven through birch itself:
#    move onto the file row and press enter, which runs the adapter's `preview`.
for ws in "$beta" "$alpha"; do
    cli workspace select "$ws" >/dev/null; settle 3
    want=$(selected)
    before=$(previews)
    cli send-key --surface "$dock" down >/dev/null; settle 1
    cli send-key --surface "$dock" enter >/dev/null; settle 3
    opened=$(previews | grep -vxF "$before" | head -1 | awk '{print $2}')
    check "a preview from the dock birch opens in the selected workspace ($ws)" \
        "$want" "${opened:-none opened}"
done

# 4. `dock-socket` addresses this window's dock birch. The adapter resolves the
#    window through the bare `cmux` CLI, so it needs the socket in its
#    environment — set per-invocation, never exported, since an empty value
#    would silently fall back to a real cmux.
sock=$(CMUX_SOCKET_PATH="$("$testcmux" sock)" "$adapter" dock-socket 2>/dev/null || true)
case "$sock" in
/*) got=$([ -S "$sock" ] && echo yes || echo "no socket at $sock") ;;
*)  got="no path printed" ;;
esac
check "dock-socket names an existing socket" yes "$got"

# It has to be the *live* dock birch's socket, not merely a well-formed path.
if [ -S "$sock" ] && "$birch" ctl --socket "$sock" get-path >/dev/null 2>&1; then got=yes; else got=no; fi
check "dock-socket addresses the running dock birch" yes "$got"

# 5. Reverse-reveal: a path inside a collapsed folder becomes visible. The
#    folder is made now, so the watcher picks it up and birch shows it closed —
#    birch never auto-expands.
cli workspace select "$alpha" >/dev/null; settle 3
mkdir -p "$work/ws-alpha/nested"; : > "$work/ws-alpha/nested/deep-file.txt"
settle 3
# An empty capture would satisfy "does not contain deep-file.txt" without
# meaning anything, so require the folder row to be on screen too.
screen=$(cli capture-pane --surface "$dock" 2>/dev/null)
case "$screen" in
*deep-file.txt*) pre=visible ;;
*nested*)        pre=hidden ;;
*)               pre="nothing on screen" ;;
esac
check "the nested file starts hidden, its folder shown" hidden "$pre"
"$birch" ctl --socket "$sock" reveal "$work/ws-alpha/nested/deep-file.txt" >/dev/null 2>&1 || true
settle 2
case "$(cli capture-pane --surface "$dock" 2>/dev/null)" in
*deep-file.txt*) got=visible ;;
*)               got=hidden ;;
esac
check "reveal through dock-socket shows it" visible "$got"

trap - EXIT INT TERM
"$testcmux" stop
printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
