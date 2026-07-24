//! The birch binary: flag parsing, terminal lifecycle, and the wiring between
//! input, sources, the watcher, the git worker, and the render loop
//! (ADR 0004).

mod app;
mod ctl;
mod ctl_client;
mod term;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use birch_core::files_source::FilesSource;
use birch_core::git::{self, GitWorker};
use birch_core::search::IndexWorker;
use birch_core::watcher::FsWatcher;
use birch_core::{OpenCmd, OpenMode, Settings, SourceCmd};
use clap::Parser;

use crate::app::Mode;

/// Lean and beautiful interactive file tree for the terminal.
///
/// Run `birch ctl --help` to control a running instance over its socket.
///
/// Tip: mouse capture disables the terminal's drag-to-copy; hold Shift while
/// dragging to select text natively.
#[derive(Parser, Debug)]
#[command(name = "birch", version, about)]
struct Cli {
    /// Root directory of the tree (default: current directory).
    dir: Option<PathBuf>,

    /// Disable Nerd Font icons.
    #[arg(long)]
    no_icons: bool,

    /// Sort files before directories.
    #[arg(long)]
    files_first: bool,

    /// Hide hidden (dot) files.
    #[arg(long)]
    hide_hidden: bool,

    /// Show noise entries (.git, .DS_Store, …).
    #[arg(long)]
    show_noise: bool,

    /// Disable mouse support.
    #[arg(long)]
    no_mouse: bool,

    /// Disable git status integration.
    #[arg(long)]
    no_git: bool,

    /// Hide gitignored files (default: shown dimmed).
    #[arg(long)]
    hide_ignored: bool,

    /// Disable compact single-child folder chains.
    #[arg(long)]
    no_compact: bool,

    /// Bind the control socket exactly here (host rendezvous) instead of
    /// the default per-instance addressing.
    #[arg(long, value_name = "path")]
    socket: Option<PathBuf>,

    /// Picker mode: search filters, Enter prints the selection (file or
    /// dir) to stdout and exits.
    #[arg(long)]
    pick: bool,

    /// Do not bind the control socket.
    #[arg(long, conflicts_with = "socket")]
    no_socket: bool,

    /// Command template for opening files: {} is the path (appended when
    /// absent). Default: $VISUAL, else $EDITOR, else the platform opener.
    #[arg(long, value_name = "template")]
    open_cmd: Option<String>,

    /// The open command is fire-and-forget: spawn it detached from the tty
    /// (null stdio) instead of handing the terminal over and waiting. For
    /// host-adapter open commands; terminal editors must not use this.
    #[arg(long, requires = "open_cmd")]
    open_detached: bool,
}

pub enum AppEvent {
    Input(crossterm::event::Event),
    Source(birch_core::SourceEvent),
    Git(birch_core::git::GitEvent),
    Fs(birch_core::watcher::WatchEvent),
    Index(birch_core::search::IndexEvent),
    Ctl(ctl::CtlRequest),
    /// SIGHUP/SIGTERM: quit through the normal path (state saved, terminal
    /// restored, socket unlinked).
    Shutdown,
}

/// Forwards a typed worker-event channel into the unified app channel.
fn forward<T, F>(rx: mpsc::Receiver<T>, tx: mpsc::Sender<AppEvent>, wrap: F)
where
    T: Send + 'static,
    F: Fn(T) -> AppEvent + Send + 'static,
{
    thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if tx.send(wrap(event)).is_err() {
                break;
            }
        }
    });
}

fn main() -> ExitCode {
    // `birch ctl <verb>` controls a running instance; anything else launches the
    // tree. Dispatch by hand: the launch form's optional [DIR] positional can't
    // be cleanly disambiguated from a clap subcommand.
    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "ctl") {
        return ctl_client::run(&args[2..]);
    }

    let cli = Cli::parse();

    let root = cli.dir.unwrap_or_else(|| PathBuf::from("."));
    let root = match root.canonicalize() {
        Ok(root) if root.is_dir() => root,
        Ok(other) => {
            eprintln!("birch: {} is not a directory", other.display());
            return ExitCode::FAILURE;
        }
        Err(e) => {
            eprintln!("birch: cannot open {}: {e}", root.display());
            return ExitCode::FAILURE;
        }
    };

    let settings = Settings {
        icons: !cli.no_icons,
        files_first: cli.files_first,
        show_hidden: !cli.hide_hidden,
        show_noise: cli.show_noise,
        mouse: !cli.no_mouse,
        git: !cli.no_git,
        show_ignored: !cli.hide_ignored,
        compact: !cli.no_compact,
    };

    let open_cmd = match cli.open_cmd.as_deref() {
        Some(template) => match OpenCmd::from_template(template) {
            Ok(mut cmd) => {
                if cli.open_detached {
                    cmd.mode = OpenMode::Detached;
                }
                cmd
            }
            Err(e) => {
                eprintln!("birch: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => OpenCmd::default_cmd(),
    };

    let (event_tx, event_rx) = mpsc::channel::<AppEvent>();

    // Files source.
    let (source_cmd_tx, source_cmd_rx) = mpsc::channel::<SourceCmd>();
    let (source_event_tx, source_event_rx) = mpsc::channel();
    let _source = FilesSource::spawn(source_cmd_rx, source_event_tx);
    forward(source_event_rx, event_tx.clone(), AppEvent::Source);

    // Filesystem watcher.
    let (watch_cmd_tx, watch_cmd_rx) = mpsc::channel();
    let (watch_event_tx, watch_event_rx) = mpsc::channel();
    let _watcher = FsWatcher::spawn(watch_cmd_rx, watch_event_tx);
    forward(watch_event_rx, event_tx.clone(), AppEvent::Fs);

    // Search index worker (builds lazily, on the first Rebuild command).
    let (index_cmd_tx, index_cmd_rx) = mpsc::channel();
    let (index_event_tx, index_event_rx) = mpsc::channel();
    let _index = IndexWorker::spawn(index_cmd_rx, index_event_tx);
    forward(index_event_rx, event_tx.clone(), AppEvent::Index);

    // Git worker — always spawned (the repo can change via set-root); idle
    // until a Refresh names a repo.
    let repo_root = if settings.git {
        git::discover_repo(&root)
    } else {
        None
    };
    let (git_cmd_tx, git_cmd_rx) = mpsc::channel();
    let (git_event_tx, git_event_rx) = mpsc::channel();
    let _git = GitWorker::spawn(git_cmd_rx, git_event_tx);
    forward(git_event_rx, event_tx.clone(), AppEvent::Git);

    // Input thread. `paused` gates reads while a child owns the terminal.
    let input_paused = Arc::new(AtomicBool::new(false));
    {
        let paused = input_paused.clone();
        let event_tx = event_tx.clone();
        thread::spawn(move || {
            loop {
                if paused.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(25));
                    continue;
                }
                match crossterm::event::poll(Duration::from_millis(100)) {
                    Ok(true) => {
                        let Ok(ev) = crossterm::event::read() else {
                            break;
                        };
                        if event_tx.send(AppEvent::Input(ev)).is_err() {
                            break;
                        }
                    }
                    Ok(false) => {}
                    Err(_) => break,
                }
            }
        });
    }

    let mode = if cli.pick { Mode::Pick } else { Mode::Tree };
    let picker = mode != Mode::Tree;

    // Control socket (never in picker mode). An explicit --socket that fails
    // is fatal — the host chose the path and expects it bound; default
    // addressing degrades to a socketless instance with a warning.
    let socket = if picker || cli.no_socket {
        None
    } else {
        let explicit = cli.socket.is_some();
        match ctl::serve(cli.socket, &root, event_tx.clone()) {
            Ok(handle) => Some(handle),
            Err(e) if explicit => {
                eprintln!("birch: cannot bind the control socket: {e}");
                return ExitCode::FAILURE;
            }
            Err(e) => {
                eprintln!("birch: control socket unavailable: {e}");
                None
            }
        }
    };

    // SIGHUP/SIGTERM → the normal quit path; a second signal during a hung
    // shutdown force-exits instead of being absorbed.
    {
        let event_tx = event_tx.clone();
        match signal_hook::iterator::Signals::new([
            signal_hook::consts::SIGHUP,
            signal_hook::consts::SIGTERM,
        ]) {
            Ok(mut signals) => {
                thread::spawn(move || {
                    let mut delivered = false;
                    for _signal in signals.forever() {
                        if delivered {
                            std::process::exit(1);
                        }
                        delivered = true;
                        let _ = event_tx.send(AppEvent::Shutdown);
                    }
                });
            }
            Err(e) => eprintln!("birch: signal handling unavailable: {e}"),
        }
    }

    let mouse = settings.mouse;
    // Picker mode renders on stderr: stdout carries only the picked path.
    let mut terminal = match term::enter(mouse, picker) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("birch: cannot set up the terminal: {e}");
            return ExitCode::FAILURE;
        }
    };
    let result = app::run(
        &mut terminal,
        app::AppWiring {
            root,
            settings,
            open_cmd,
            mode,
            events: event_rx,
            source_cmds: source_cmd_tx,
            watch_cmds: watch_cmd_tx,
            index_cmds: index_cmd_tx,
            git_cmds: git_cmd_tx,
            repo_root,
            socket,
            input_paused,
        },
    );
    term::leave(mouse, picker);

    match result {
        Ok(Some(picked)) => {
            println!("{}", picked.display());
            ExitCode::SUCCESS
        }
        Ok(None) if picker => ExitCode::FAILURE, // quit without a pick
        Ok(None) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("birch: {e}");
            ExitCode::FAILURE
        }
    }
}
