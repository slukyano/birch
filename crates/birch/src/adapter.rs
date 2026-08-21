//! Running a contrib host adapter that ships as a binary.
//!
//! The adapters are shell scripts under `contrib/` — that is where they are
//! read, copied and modified, and `docs/design.md` treats them as examples. But
//! a script bundled beside a release lands in `share/birch/`, not on `PATH`, so
//! `birch-cmux dock-run` in a cmux `dock.json` could only ever be an absolute
//! path into someone's install. Embedding each script in a tiny binary of the
//! same name puts it on `PATH` through every channel — Homebrew, the shell
//! installer, and `cargo install`, which shipped no adapters at all.

/// Runs `script` as `sh -c`, forwarding this process's arguments, and does not
/// return.
#[cfg(unix)]
pub fn run(script: &str) -> ! {
    use std::os::unix::process::CommandExt;

    // `sh -c SCRIPT NAME ARGS...` sets `$0` to NAME. The adapters bake `$0` into
    // birch's `--open-cmd` and re-exec themselves through it, so it has to be
    // something that invokes this adapter again.
    //
    // Our own argv[0] is the right answer, and better than `current_exe()`: the
    // scripts already resolve a bare name through `command -v`, which lands on
    // the stable `…/bin/birch-cmux`, while `current_exe()` resolves symlinks and
    // would pin the callback to a versioned path — `…/Cellar/birch/0.2.0/bin/…`
    // for a Homebrew install, which `brew cleanup` deletes out from under a
    // still-running dock birch. `current_exe()` is only the fallback for an
    // argv[0] that is missing or empty.
    let argv0 = std::env::args_os()
        .next()
        .filter(|a| !a.is_empty())
        .or_else(|| std::env::current_exe().ok().map(Into::into))
        .unwrap_or_default();

    // `exec`, not spawn-and-wait: the adapters rely on being the process the
    // host signals — a dock control dies with its pty's SIGHUP — and on their
    // own exit status reaching the caller.
    let error = std::process::Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .arg(argv0)
        .args(std::env::args_os().skip(1))
        .exec();

    // `exec` only returns when it failed.
    eprintln!("birch: could not run /bin/sh: {error}");
    std::process::exit(126)
}

#[cfg(not(unix))]
pub fn run(_script: &str) -> ! {
    eprintln!("birch: the host adapters need a POSIX shell");
    std::process::exit(1)
}
