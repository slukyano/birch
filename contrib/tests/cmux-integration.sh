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
adapter="$contrib/birch-cmux"
birch="${1:-$repo/target/debug/birch}"

[ -x "$birch" ] || { echo "no birch at $birch (cargo build first)" >&2; exit 1; }

work="${TESTCMUX_HOME:-${TMPDIR:-/tmp}/birch-testcmux}/work"
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
"$testcmux" start >/dev/null

alpha=$(cli workspace create --name AlphaWs --cwd "$work/ws-alpha" 2>/dev/null | awk '{print $2}')
beta=$(cli workspace create --name BetaWs  --cwd "$work/ws-beta"  2>/dev/null | awk '{print $2}')
settle 2

# A Dock surface, then the adapter inside it — the same entrypoint dock.json calls.
dock=$(cli new-surface --placement dock --focus false 2>/dev/null | awk '{print $2}')
settle 2
cli workspace select "$alpha" >/dev/null
settle 1
cli send --surface "$dock" "clear; BIRCH=$birch $adapter dock-run\n" >/dev/null
settle 5

# 1. birch is in the sidebar, rooted at the selected workspace.
root_line() { cli capture-pane --surface "$dock" 2>/dev/null | sed -n '1p'; }
case "$(root_line)" in
*ws-alpha*) check "birch runs in the Dock, rooted at the selected workspace" yes yes ;;
*)          check "birch runs in the Dock, rooted at the selected workspace" yes "no (root line: $(root_line))" ;;
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

"$testcmux" stop
printf '\n%d passed, %d failed\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
