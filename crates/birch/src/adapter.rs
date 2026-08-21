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

    // `sh -c SCRIPT NAME ARGS...` sets `$0` to NAME. It has to be this
    // executable's own path: the adapters bake `$0` into birch's `--open-cmd`
    // and re-exec themselves through it, so the script's own name would send
    // those callbacks nowhere.
    let argv0 = std::env::current_exe()
        .ok()
        .and_then(|p| p.into_os_string().into_string().ok())
        .unwrap_or_else(|| std::env::args().next().unwrap_or_default());

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
