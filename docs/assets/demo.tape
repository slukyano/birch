# Regenerates docs/assets/demo.gif with vhs (https://github.com/charmbracelet/vhs):
#
#   vhs docs/assets/demo.tape
#
# Prerequisites:
#   - `birch` on PATH (e.g. `mise run dev`, or `brew install slukyano/tap/birch`).
#   - For Nerd Font icons: install a Nerd Font, set FontFamily to it below, and drop
#     `--no-icons`. Without one, `--no-icons` keeps the tree clean (no missing-glyph boxes).

Output "docs/assets/demo.gif"

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 18
Set Width 860
Set Height 470
Set Padding 18
Set Theme "Catppuccin Mocha"

# Build a small sample project (git repo with a modified + an untracked file, and an
# ignored target/) so the tree shows real git status and ignored-dimming.
Hide
Type `rm -rf /tmp/acme; mkdir -p /tmp/acme/src/cli /tmp/acme/src/core /tmp/acme/tests /tmp/acme/docs; cd /tmp/acme` Enter
Type `: > README.md Cargo.toml src/main.rs src/cli/args.rs src/cli/parse.rs src/core/engine.rs src/core/state.rs tests/integration.rs docs/design.md` Enter
Type `printf 'target/\n' > .gitignore; mkdir target; : > target/ignored.bin` Enter
Type `git init -q && git add -A && git -c user.email=a@b.c -c user.name=demo commit -qm init` Enter
Type `printf 'fn main() {}\n' > src/main.rs; : > src/core/cache.rs; clear` Enter
Show

Sleep 600ms
Type "birch --no-icons"
Enter
Sleep 1500ms
Down@400ms 2
Right
Sleep 700ms
Down@400ms 2
Right
Sleep 900ms
Sleep 500ms
Type@220ms "engine"
Sleep 300ms
Sleep 1300ms
Escape
Sleep 700ms
Ctrl+C
Sleep 700ms
