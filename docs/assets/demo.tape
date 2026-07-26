# Regenerates docs/assets/demo.gif with vhs (https://github.com/charmbracelet/vhs):
#
#   vhs docs/assets/demo.tape
#
# Prerequisites:
#   - `birch` on PATH (e.g. `mise run dev`, or `brew install slukyano/tap/birch`).
#   - Needs a Nerd Font (FontFamily below) for the file-type icons. Without one, add
#     `--no-icons` to the birch command and use a plain monospace family (e.g. "Menlo").

Output "docs/assets/demo.gif"

Set Shell "bash"
Set FontFamily "JetBrainsMono Nerd Font Mono"
Set FontSize 18
Set Width 860
Set Height 470
Set Padding 18
Set Theme "Catppuccin Mocha"

# Build a small sample git repo (a modified + an untracked file, an ignored target/), clear
# birch's saved state, and launch birch — all hidden — so the FIRST visible frame is the
# populated tree and the walkthrough is deterministic.
Hide
Type `rm -rf /tmp/acme; mkdir -p /tmp/acme/src/cli /tmp/acme/src/core /tmp/acme/tests /tmp/acme/docs; cd /tmp/acme` Enter
Type `touch README.md Cargo.toml src/main.rs src/cli/args.rs src/cli/parse.rs src/core/engine.rs src/core/state.rs tests/integration.rs docs/design.md` Enter
Type `printf 'target/\n' > .gitignore; mkdir target; touch target/ignored.bin` Enter
Type `git init -q && git add -A && git -c user.email=a@b.c -c user.name=demo commit -qm init` Enter
Type `printf 'fn main() {}\n' > src/main.rs; touch src/core/cache.rs; rm -rf "${XDG_CACHE_HOME:-$HOME/.cache}/birch"; clear` Enter
Type "birch" Enter
Sleep 1800ms
Show

Sleep 900ms
Down@400ms 2
Right
Sleep 700ms
Down@400ms 2
Right
Sleep 1000ms
Type@220ms "engine"
Sleep 1600ms
Escape
Sleep 700ms
Ctrl+C
Sleep 500ms
