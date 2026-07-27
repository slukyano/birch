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
use birch_core::{Config, OpenCmd, OpenMode, Settings, SourceCmd};
use clap::Parser;

use crate::app::Mode;

/// Modern interactive file tree for the terminal.
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

    /// Use this config file instead of the discovered one (for tests and
    /// alternate setups). Default: $XDG_CONFIG_HOME/birch/birch.toml, else
    /// ~/.config/birch/birch.toml.
    #[arg(long, value_name = "path")]
    config: Option<PathBuf>,

    // Bidirectional toggles (ADR 0022): each setting has both directions so
    // the CLI can override the config file either way. `overrides_with` makes
    // the last flag on the command line win; the counterpart matching the
    // built-in default direction is hidden from --help to keep it uncluttered.
    /// Enable Nerd Font icons.
    #[arg(long, overrides_with = "no_icons", hide = true)]
    icons: bool,
    /// Disable Nerd Font icons.
    #[arg(long, overrides_with = "icons")]
    no_icons: bool,

    /// Show hidden (dot) files.
    #[arg(long, overrides_with = "hide_hidden", hide = true)]
    show_hidden: bool,
    /// Hide hidden (dot) files.
    #[arg(long, overrides_with = "show_hidden")]
    hide_hidden: bool,

    /// Show noise entries (.git, .DS_Store, …).
    #[arg(long, overrides_with = "hide_noise")]
    show_noise: bool,
    /// Hide noise entries (.git, .DS_Store, …).
    #[arg(long, overrides_with = "show_noise", hide = true)]
    hide_noise: bool,

    /// Enable mouse support.
    #[arg(long, overrides_with = "no_mouse", hide = true)]
    mouse: bool,
    /// Disable mouse support.
    #[arg(long, overrides_with = "mouse")]
    no_mouse: bool,

    /// Enable git status integration.
    #[arg(long, overrides_with = "no_git", hide = true)]
    git: bool,
    /// Disable git status integration.
    #[arg(long, overrides_with = "git")]
    no_git: bool,

    /// Show gitignored files dimmed.
    #[arg(long, overrides_with = "hide_ignored", hide = true)]
    show_ignored: bool,
    /// Hide gitignored files (default: shown dimmed).
    #[arg(long, overrides_with = "show_ignored")]
    hide_ignored: bool,

    /// Enable compact single-child folder chains.
    #[arg(long, overrides_with = "no_compact", hide = true)]
    compact: bool,
    /// Disable compact single-child folder chains.
    #[arg(long, overrides_with = "compact")]
    no_compact: bool,

    /// Visual theme (colors, glyphs, guides). Overrides the config `theme`.
    #[arg(long, value_enum)]
    theme: Option<ThemeArg>,

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

/// Launch-flag mirror of `birch_core::ThemeId` (core stays clap-free).
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum ThemeArg {
    Birch,
    Vscode,
    Jetbrains,
    Xcode,
    Retro,
    Plain,
}

impl From<ThemeArg> for birch_core::ThemeId {
    fn from(value: ThemeArg) -> Self {
        use birch_core::ThemeId;
        match value {
            ThemeArg::Birch => ThemeId::Birch,
            ThemeArg::Vscode => ThemeId::Vscode,
            ThemeArg::Jetbrains => ThemeId::Jetbrains,
            ThemeArg::Xcode => ThemeId::Xcode,
            ThemeArg::Retro => ThemeId::Retro,
            ThemeArg::Plain => ThemeId::Plain,
        }
    }
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

    // Precedence: Settings::default() → config → CLI flags (ADR 0022). The
    // config warning goes to stderr here, before the terminal is taken.
    let (config, config_warning) = Config::load(cli.config.as_deref());
    if let Some(warning) = config_warning {
        eprintln!("{warning}");
    }

    let mut settings = Settings::default();
    config.apply_to(&mut settings);
    // Resolve each bidirectional toggle to Option<bool> (last-flag-wins is
    // handled by clap's `overrides_with`) and apply only when the user set it.
    let flag = |on: bool, off: bool| -> Option<bool> {
        if on {
            Some(true)
        } else if off {
            Some(false)
        } else {
            None
        }
    };
    if let Some(v) = flag(cli.icons, cli.no_icons) {
        settings.icons = v;
    }
    if let Some(v) = flag(cli.show_hidden, cli.hide_hidden) {
        settings.show_hidden = v;
    }
    if let Some(v) = flag(cli.show_noise, cli.hide_noise) {
        settings.show_noise = v;
    }
    if let Some(v) = flag(cli.mouse, cli.no_mouse) {
        settings.mouse = v;
    }
    if let Some(v) = flag(cli.git, cli.no_git) {
        settings.git = v;
    }
    if let Some(v) = flag(cli.show_ignored, cli.hide_ignored) {
        settings.show_ignored = v;
    }
    if let Some(v) = flag(cli.compact, cli.no_compact) {
        settings.compact = v;
    }
    if let Some(theme) = cli.theme {
        settings.theme = theme.into();
    }

    // Open command: --open-cmd wins, else config `open-cmd`, else the built-in.
    let open_cmd_template = cli.open_cmd.as_deref().or(config.open_cmd.as_deref());
    let open_cmd = match open_cmd_template {
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
