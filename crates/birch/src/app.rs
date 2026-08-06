//! The app loop: one `recv()` over the unified event channel followed by a
//! drain of whatever else has queued, so a *batch* of events costs one redraw
//! (ADR 0024); deltas mutate the tree, input becomes actions, every iteration
//! reconciles watches, requests peek-loads, persists expansion changes, and
//! redraws.

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use birch_core::git::{self, GitCmd, GitEvent, GitState};
use birch_core::protocol::{PathForm, Request, Response, SettingKey, SettingValue, Verb};
use birch_core::search::{self, IndexCmd, IndexEvent, Match, SearchIndex, search};
use birch_core::watcher::{WatchCmd, WatchEvent};
use birch_core::{
    Filter, NodeKind, OpenCmd, OpenMode, Settings, SourceCmd, SourceEvent, ThemeId, Tree,
    TreeDelta, persist, settings,
};
use birch_tui::flat_view::{self, Decor, FlatView, NavEffect, Row};
use birch_tui::input::{self, InputAction};
use birch_tui::render;
use birch_tui::theme::Theme;
use ratatui::layout::Rect;

use crate::ctl::{CtlRequest, SocketHandle};
use crate::{AppEvent, term};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Tree,
    Pick,
}

impl Mode {
    fn is_pick(self) -> bool {
        self == Mode::Pick
    }
}

pub struct AppWiring {
    pub root: PathBuf,
    pub settings: Settings,
    pub open_cmd: OpenCmd,
    pub mode: Mode,
    pub events: Receiver<AppEvent>,
    pub source_cmds: Sender<SourceCmd>,
    pub watch_cmds: Sender<WatchCmd>,
    pub index_cmds: Sender<IndexCmd>,
    pub git_cmds: Sender<GitCmd>,
    /// Present when a repo was discovered and git is enabled.
    pub repo_root: Option<PathBuf>,
    /// The control socket, when one was bound (never in picker mode).
    pub socket: Option<SocketHandle>,
    /// The compiled glob filter (task 027), when `--filter` was given.
    pub filter: Option<Filter>,
    pub input_paused: Arc<AtomicBool>,
}

/// Active fuzzy search (ADR 0009). The pane keeps its tree and steps over the
/// matches; everything else dims and stops being selectable (ADR 0023). Matches
/// are held in tree order, and `current` indexes them (063).
struct SearchState {
    query: String,
    matches: Vec<Match>,
    /// Path → matched char indices into the simple name (empty for path-mode
    /// hits, meaning whole-name highlight).
    matched_set: HashMap<PathBuf, Vec<u32>>,
    saved_selection: Option<PathBuf>,
    saved_scroll: usize,
}

struct App {
    tree: Tree,
    view: FlatView,
    settings: Settings,
    open_cmd: OpenCmd,
    mode: Mode,
    status: String,
    root: PathBuf,
    root_label: String,
    source_cmds: Sender<SourceCmd>,
    watch_cmds: Sender<WatchCmd>,
    index_cmds: Sender<IndexCmd>,
    git_cmds: Sender<GitCmd>,
    repo_root: Option<PathBuf>,
    socket: Option<SocketHandle>,
    filter: Option<Filter>,
    git_state: Option<Arc<GitState>>,
    /// The git worker answered at least once — peeks wait for it so ignored
    /// dirs are known before any auto-load fires.
    git_answered: bool,
    watched: HashSet<PathBuf>,
    requested_peeks: HashSet<PathBuf>,
    index: Option<Arc<SearchIndex>>,
    index_requested: bool,
    search: Option<SearchState>,
    /// A path being revealed: ancestors expand as loads arrive, selection
    /// lands when the path shows up (search jumps; later the socket verb).
    pending_reveal: Option<PathBuf>,
    /// Computed once at startup; the root annotation abbreviates with it.
    home: Option<PathBuf>,
    /// Dirs restored from the state file, expanded as their parents load.
    restore_expanded: HashSet<PathBuf>,
    expansion_dirty: bool,
    last_saved_expanded: Vec<PathBuf>,
    picked: Option<PathBuf>,
    input_paused: Arc<AtomicBool>,
    click_timer: input::ClickTimer,
    /// The armed press: which row a left button-down landed on, and whether it
    /// landed in the chevron zone. A click completes only when the release
    /// matches both (ADR 0025). Keyed on the real path, so a snapshot arriving
    /// between press and release cannot redirect the click.
    armed_press: Option<(PathBuf, bool)>,
    /// Row count from the last draw. A scroll needs the count and nothing
    /// else, so it is served from here instead of rebuilding every row
    /// (ADR 0024); `FlatView::reconcile` re-clamps before every draw anyway.
    rows_len: usize,
    /// Set when a child process owned the terminal, so the batch ends and the
    /// screen it left is redrawn at once (ADR 0024).
    yielded_terminal: bool,
}

/// Runs the app; in picker mode the returned path is the confirmed pick.
/// How long one batch may spend consuming already-queued events before the
/// frame is drawn regardless (ADR 0024). A budget rather than an event count:
/// it degrades with the machine's speed instead of meaning different things on
/// different hardware.
const BATCH_BUDGET: Duration = Duration::from_millis(8);

/// What the batch should do after an event was handled.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BatchStep {
    Continue,
    /// End the batch and draw (a child owned the terminal).
    Stop,
    /// End the batch and the loop.
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum BatchStop {
    Drained,
    Budget,
    Handoff,
    Quit,
}

/// Handles every event already queued, in order, until the channel is empty or
/// `deadline` passes. Never blocks: only what has *already* arrived joins the
/// batch, so an idle birch still draws one frame per event.
fn drain_batch<F>(events: &Receiver<AppEvent>, deadline: Instant, mut handle: F) -> BatchStop
where
    F: FnMut(AppEvent) -> BatchStep,
{
    loop {
        if Instant::now() >= deadline {
            return BatchStop::Budget;
        }
        let Ok(event) = events.try_recv() else {
            return BatchStop::Drained;
        };
        match handle(event) {
            BatchStep::Continue => {}
            BatchStep::Stop => return BatchStop::Handoff,
            BatchStep::Quit => return BatchStop::Quit,
        }
    }
}

pub fn run(terminal: &mut term::Term, wiring: AppWiring) -> io::Result<Option<PathBuf>> {
    let AppWiring {
        root,
        settings,
        open_cmd,
        mode,
        events,
        source_cmds,
        watch_cmds,
        index_cmds,
        git_cmds,
        repo_root,
        socket,
        filter,
        input_paused,
    } = wiring;

    let mut app = App {
        tree: Tree::new(root.clone()),
        view: FlatView::default(),
        root_label: root.display().to_string(),
        settings,
        open_cmd,
        mode,
        status: String::new(),
        root: root.clone(),
        source_cmds,
        watch_cmds,
        index_cmds,
        git_cmds,
        repo_root,
        socket,
        filter,
        git_state: None,
        git_answered: false,
        watched: HashSet::new(),
        requested_peeks: HashSet::new(),
        index: None,
        index_requested: false,
        search: None,
        pending_reveal: None,
        home: std::env::home_dir(),
        restore_expanded: HashSet::new(),
        expansion_dirty: false,
        last_saved_expanded: Vec::new(),
        picked: None,
        input_paused,
        click_timer: input::ClickTimer::default(),
        armed_press: None,
        rows_len: 0,
        yielded_terminal: false,
    };

    if app.mode == Mode::Tree {
        app.load_persisted();
    } else {
        app.request_index(); // the picker's primary function needs it now
    }
    app.tree.set_expanded(&root, true);
    app.requested_peeks.insert(root.clone());
    let _ = app.source_cmds.send(SourceCmd::Expand(root));
    app.refresh_git();

    // Draw-loop errors still flow through save + socket cleanup.
    let mut loop_result = app.finish_iteration(terminal);
    while loop_result.is_ok() {
        let Ok(event) = events.recv() else { break };
        if app.handle(terminal, &events, event) {
            break;
        }
        // One frame per *batch* (ADR 0024): whatever else has already queued
        // is handled first, in order, and the whole batch shares one redraw.
        let stopped = drain_batch(&events, Instant::now() + BATCH_BUDGET, |event| {
            if app.handle(terminal, &events, event) {
                BatchStep::Quit
            } else if app.yielded_terminal {
                // A child owned the tty; queued events must not run behind it,
                // and the screen it left needs the redraw now.
                BatchStep::Stop
            } else {
                BatchStep::Continue
            }
        });
        if stopped == BatchStop::Quit {
            break;
        }
        app.yielded_terminal = false;
        loop_result = app.finish_iteration(terminal);
    }
    if app.mode == Mode::Tree {
        app.save_persisted(true);
    }
    if let Some(handle) = &app.socket {
        handle.cleanup();
    }
    loop_result?;
    Ok(app.picked)
}

impl App {
    /// Returns true on quit.
    fn handle(
        &mut self,
        terminal: &mut term::Term,
        events: &Receiver<AppEvent>,
        event: AppEvent,
    ) -> bool {
        match event {
            AppEvent::Source(ev) => self.handle_source(ev),
            AppEvent::Git(GitEvent::State(state)) => {
                self.git_state = state;
                self.git_answered = true;
            }
            AppEvent::Fs(WatchEvent::Dirty(dirs)) => self.handle_dirty(dirs),
            AppEvent::Index(IndexEvent::Index(index)) => {
                self.index = Some(index);
                self.rematch();
            }
            AppEvent::Ctl(ctl) => return self.handle_ctl(terminal, events, ctl),
            AppEvent::Shutdown => return true,
            AppEvent::Input(raw) => return self.handle_input(terminal, events, raw),
        }
        false
    }

    fn handle_source(&mut self, event: SourceEvent) {
        match event {
            SourceEvent::Deltas(deltas) => {
                for delta in deltas {
                    if let TreeDelta::Snapshot { dir, .. } = &delta {
                        self.requested_peeks.remove(dir);
                    }
                    self.tree.apply(delta);
                }
            }
            SourceEvent::Message(message) => self.status = message,
        }
    }

    /// Restored expansion cascades down as listings arrive (persistence).
    /// Runs per iteration and — like peeks — waits for the first git answer,
    /// so a dir that became ignored since the save is never auto-expanded.
    fn process_restores(&mut self) {
        if self.restore_expanded.is_empty() {
            return;
        }
        if self.settings.git && self.repo_root.is_some() && !self.git_answered {
            return;
        }
        let mut ready = Vec::new();
        let mut stale = Vec::new();
        for path in &self.restore_expanded {
            if self.tree.node_at(path).is_some_and(|n| n.kind.is_dir()) {
                ready.push(path.clone());
            } else if path
                .parent()
                .and_then(|p| self.tree.node_at(p))
                .is_some_and(|n| n.is_loaded())
            {
                // The parent listing arrived without this entry: it no longer
                // exists (or is no longer a dir) — drop it, don't leak.
                stale.push(path.clone());
            }
        }
        for path in stale {
            self.restore_expanded.remove(&path);
        }
        for path in ready {
            self.restore_expanded.remove(&path);
            if self.is_ignored(&path) {
                continue; // never auto-expand ignored dirs
            }
            self.tree.set_expanded(&path, true);
            self.expansion_dirty = true;
            let _ = self.source_cmds.send(SourceCmd::Expand(path));
        }
    }

    fn is_ignored(&self, path: &Path) -> bool {
        self.git_state
            .as_deref()
            .is_some_and(|g| g.is_ignored(path))
    }

    fn handle_dirty(&mut self, dirs: Vec<PathBuf>) {
        // Any fs change may change git status; dirs the tree displays get a
        // one-level re-scan (this includes an expanded .git under
        // --show-noise — dirs outside the tree resolve to no node).
        for dir in dirs {
            if self.tree.node_at(&dir).is_some_and(|n| n.is_loaded()) {
                let _ = self.source_cmds.send(SourceCmd::Expand(dir));
            }
        }
        self.refresh_git();
        // Once an index was requested it must track the filesystem, even if
        // the first build has not landed yet.
        if self.index_requested {
            self.send_index_rebuild();
        }
    }

    fn handle_input(
        &mut self,
        terminal: &mut term::Term,
        events: &Receiver<AppEvent>,
        raw: crossterm::event::Event,
    ) -> bool {
        let Some(action) = input::map_event(&raw, self.settings.mouse) else {
            return false;
        };
        if action == InputAction::Quit {
            return true;
        }
        self.status.clear(); // status messages are transient

        // Search editing works the same in both modes.
        match action {
            InputAction::Char(c) => {
                self.search_push(c);
                return false;
            }
            InputAction::Backspace => {
                self.search_pop();
                return false;
            }
            InputAction::Esc => {
                return self.on_esc();
            }
            _ => {}
        }

        // A scroll reads the row *count* and nothing else, so it never
        // rebuilds the rows (ADR 0024). This is the hot path under a wheel
        // burst; everything below it walks the tree.
        if let Some(delta) = match action {
            InputAction::ScrollUp => Some(-(self.settings.scroll_lines as isize)),
            InputAction::ScrollDown => Some(self.settings.scroll_lines as isize),
            _ => None,
        } {
            let viewport = render::tree_viewport_height(area(terminal));
            self.scroll_rows(delta, viewport);
            return false;
        }

        let rows = self.rows();
        // With a live search, ↑/↓ cycle the matches — in both modes now
        // (ADR 0023): the picker is the same tree, so it steps the same way.
        if let Some(state) = &self.search
            && !state.matches.is_empty()
            && matches!(action, InputAction::Up | InputAction::Down)
        {
            self.cycle_match(action == InputAction::Down);
            return false;
        }
        let effect = match action {
            InputAction::Up => {
                self.view.move_by(&rows, -1);
                NavEffect::None
            }
            InputAction::Down => {
                self.view.move_by(&rows, 1);
                NavEffect::None
            }
            InputAction::Right => self.view.on_right(&mut self.tree, &rows),
            InputAction::Left => {
                self.view.on_left(&mut self.tree, &rows);
                NavEffect::None
            }
            InputAction::Enter => self.activate(&rows, None),
            // The press only arms — nothing is selected, toggled, or opened
            // until the release lands on the same row and zone (ADR 0025).
            InputAction::Press { column, row } => {
                self.armed_press = render::hit_test(
                    &rows,
                    &self.view,
                    &self.settings,
                    area(terminal),
                    column,
                    row,
                )
                .map(|(idx, on_chevron)| (rows[idx].path.clone(), on_chevron));
                NavEffect::None
            }
            InputAction::Release { column, row } => {
                let hit = render::hit_test(
                    &rows,
                    &self.view,
                    &self.settings,
                    area(terminal),
                    column,
                    row,
                );
                self.resolve_release(&rows, hit, Instant::now())
            }
            // Scrolling returned above, from the count-only fast path.
            InputAction::ScrollUp
            | InputAction::ScrollDown
            | InputAction::Redraw
            | InputAction::Quit
            | InputAction::Char(_)
            | InputAction::Backspace
            | InputAction::Esc => NavEffect::None,
        };
        // These actions may toggle expansion; the saver diffs the actual set
        // and skips no-op writes, so over-marking is cheap.
        if matches!(
            action,
            InputAction::Right
                | InputAction::Left
                | InputAction::Enter
                | InputAction::Release { .. }
        ) {
            self.expansion_dirty = true;
        }
        match effect {
            NavEffect::None => {}
            NavEffect::Message(message) => self.status = message,
            NavEffect::RequestExpand(path) => {
                let _ = self.source_cmds.send(SourceCmd::Expand(path));
            }
            NavEffect::Open(path) => {
                if self.mode.is_pick() {
                    // Unreachable by construction (activate picks first in
                    // picker mode), kept as a guard: never exec in a picker.
                    self.picked = Some(path);
                } else {
                    self.perform_open(terminal, events, &path);
                }
            }
        }
        self.picked.is_some()
    }

    /// The one open execution path — hotkeys, mouse, and the socket verb all
    /// land here (the action layer is shared by design).
    fn perform_open(
        &mut self,
        terminal: &mut term::Term,
        events: &Receiver<AppEvent>,
        path: &Path,
    ) {
        match self.open_cmd.mode {
            OpenMode::Detached => self.open_detached(path),
            OpenMode::Terminal => {
                self.open_in_terminal(terminal, path);
                // Events read before the handover completed are stale:
                // apply everything except old input.
                while let Ok(pending) = events.try_recv() {
                    match pending {
                        AppEvent::Input(_) => {}
                        other => {
                            self.handle(terminal, events, other);
                        }
                    }
                }
            }
        }
    }

    // ---- control socket verbs (ADR 0011) ----

    /// Returns true on quit. The reply is sent before any long-running work
    /// (a terminal editor must not block the client).
    fn handle_ctl(
        &mut self,
        terminal: &mut term::Term,
        events: &Receiver<AppEvent>,
        ctl: CtlRequest,
    ) -> bool {
        let CtlRequest { request, reply } = ctl;
        let (response, effect) = self.ctl_response(request);
        let _ = reply.send(response);
        match effect {
            CtlEffect::None => false,
            CtlEffect::Quit => true,
            CtlEffect::Open(path) => {
                self.perform_open(terminal, events, &path);
                false
            }
        }
    }

    /// Verb execution, separated from transport so it is testable without a
    /// terminal. The effect is what must happen after the reply is sent.
    fn ctl_response(&mut self, request: Request) -> (Response, CtlEffect) {
        let mut effect = CtlEffect::None;
        let response = match request.verb {
            Verb::Reveal => match request.path {
                Some(path) => match resolve_within_root(&self.root, &path) {
                    Some(target) => {
                        self.reveal(target);
                        Response::ok(None)
                    }
                    None => Response::err("path is outside the root"),
                },
                None => Response::err("reveal needs a path"),
            },
            Verb::GetPath => match self.view.selection.clone() {
                Some(sel) => {
                    let data = match request.form.unwrap_or_default() {
                        PathForm::Name => sel
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| sel.display().to_string()),
                        PathForm::Rel => {
                            let rel = sel.strip_prefix(&self.root).unwrap_or(&sel);
                            if rel.as_os_str().is_empty() {
                                ".".into() // the root itself
                            } else {
                                rel.display().to_string()
                            }
                        }
                        PathForm::Abs => sel.display().to_string(),
                    };
                    Response::ok(Some(data))
                }
                None => Response::err("no selection"),
            },
            Verb::GetRoot => Response::ok(Some(self.root.display().to_string())),
            Verb::Set => self.handle_set(request.setting, request.value.as_deref()),
            Verb::SetRoot => match request.path {
                Some(path) => self.set_root(path),
                None => Response::err("set-root needs a path"),
            },
            Verb::Open => match self.view.selection.clone() {
                Some(sel) => {
                    if self.tree.node_at(&sel).is_some_and(|n| n.kind.is_dir()) {
                        // Open on a dir behaves like Enter: expand.
                        if self.tree.set_expanded(&sel, true)
                            && !self.tree.node_at(&sel).is_some_and(|n| n.is_loaded())
                        {
                            let _ = self.source_cmds.send(SourceCmd::Expand(sel));
                        }
                    } else {
                        effect = CtlEffect::Open(sel);
                    }
                    Response::ok(None)
                }
                None => Response::err("no selection"),
            },
            Verb::Quit => {
                effect = CtlEffect::Quit;
                Response::ok(None)
            }
        };
        (response, effect)
    }

    fn handle_set(&mut self, key: Option<SettingKey>, value: Option<&str>) -> Response {
        let (Some(key), Some(value)) = (key, value) else {
            return Response::err("set needs a setting and a value");
        };
        // Theme is a theme-id string, not the on/off SettingValue every other
        // key parses. The redraw at the end of the loop applies it live.
        // Numeric settings are parsed before SettingValue, as Theme is: their
        // value is not on/off/toggle.
        if let SettingKey::ScrollLines = key {
            return match value.parse::<u8>() {
                Ok(n) if (settings::SCROLL_LINES_MIN..=settings::SCROLL_LINES_MAX).contains(&n) => {
                    self.settings.scroll_lines = n;
                    Response::ok(None)
                }
                _ => Response::err(format!(
                    "scroll-lines must be a whole number from {} to {}",
                    settings::SCROLL_LINES_MIN,
                    settings::SCROLL_LINES_MAX
                )),
            };
        }
        if let SettingKey::Theme = key {
            return match value.parse::<ThemeId>() {
                Ok(id) => {
                    self.settings.theme = id;
                    Response::ok(None)
                }
                Err(e) => Response::err(e),
            };
        }
        let Some(value) = SettingValue::parse(value) else {
            return Response::err("value must be on/off/true/false/1/0/toggle");
        };
        match key {
            SettingKey::Hidden => {
                self.settings.show_hidden = value.apply(self.settings.show_hidden);
                if self.index_requested {
                    self.send_index_rebuild();
                }
            }
            SettingKey::Ignored => {
                self.settings.show_ignored = value.apply(self.settings.show_ignored);
            }
            SettingKey::Noise => self.settings.show_noise = value.apply(self.settings.show_noise),
            SettingKey::Icons => self.settings.icons = value.apply(self.settings.icons),
            SettingKey::Compact => self.settings.compact = value.apply(self.settings.compact),
            SettingKey::Git => {
                self.settings.git = value.apply(self.settings.git);
                if self.settings.git {
                    // The repo may never have been discovered (--no-git
                    // startup) or may have changed; rediscover now.
                    self.repo_root = git::discover_repo(&self.root);
                    self.refresh_git();
                } else {
                    // Stale decorations must not keep rendering.
                    self.git_state = None;
                    self.repo_root = None;
                }
            }
            SettingKey::Scrollbar => {
                self.settings.scrollbar = value.apply(self.settings.scrollbar);
            }
            SettingKey::Theme | SettingKey::ScrollLines => {
                return Response::err("handled before value parsing");
            }
        }
        Response::ok(None)
    }

    /// Re-roots the instance (ADR 0010: any readable dir). The old root's
    /// state is saved; tree, view, search, git, and the by-root symlink all
    /// rebind to the new root.
    fn set_root(&mut self, path: PathBuf) -> Response {
        let abs = if path.is_absolute() {
            path
        } else {
            self.root.join(path)
        };
        let new_root = match abs.canonicalize() {
            Ok(root) if root.is_dir() => root,
            Ok(other) => return Response::err(format!("{} is not a directory", other.display())),
            Err(e) => return Response::err(format!("cannot open {}: {e}", abs.display())),
        };
        if new_root == self.root {
            return Response::ok(None);
        }
        if self.mode == Mode::Tree {
            self.save_persisted(true);
        }
        self.root_label = new_root.display().to_string();
        self.root = new_root.clone();
        self.tree = Tree::new(new_root.clone());
        self.view = FlatView::default();
        self.search = None;
        self.pending_reveal = None;
        self.status.clear();
        self.requested_peeks.clear();
        self.restore_expanded.clear();
        self.expansion_dirty = false;
        self.last_saved_expanded.clear();
        self.index = None;
        self.git_state = None;
        self.git_answered = false;
        self.repo_root = if self.settings.git {
            git::discover_repo(&new_root)
        } else {
            None
        };
        if self.mode == Mode::Tree {
            self.load_persisted();
        }
        self.tree.set_expanded(&new_root, true);
        self.requested_peeks.insert(new_root.clone());
        let _ = self.source_cmds.send(SourceCmd::Expand(new_root.clone()));
        self.refresh_git();
        if self.index_requested {
            self.send_index_rebuild();
        }
        if let Some(handle) = &mut self.socket {
            handle.repoint(&new_root);
        }
        Response::ok(None)
    }

    /// Click decision (ADR 0015): chevron clicks activate immediately (each
    /// press is its own toggle, and it disarms a pending double — chevron-
    /// then-name fast is a select); name clicks select, and only a completed
    /// double-click activates. Tree semantics now apply in both modes, since
    /// the picker renders the same tree (ADR 0023).
    /// Completes an armed press, or abandons it. A click acts only when the
    /// release lands on the same row *and* the same zone as the press; every
    /// other outcome does nothing and clears a pending double-click, which is
    /// how ADR 0015 already treats an intervening click (ADR 0025).
    fn resolve_release(
        &mut self,
        rows: &[Row],
        hit: Option<(usize, bool)>,
        now: Instant,
    ) -> NavEffect {
        let armed = self.armed_press.take();
        let (Some((path, armed_chevron)), Some((idx, on_chevron))) = (armed, hit) else {
            self.click_timer.disarm();
            return NavEffect::None;
        };
        if on_chevron != armed_chevron || rows.get(idx).map(|r| r.path.as_path()) != Some(&path) {
            self.click_timer.disarm();
            return NavEffect::None;
        }
        self.resolve_click(rows, idx, on_chevron, now)
    }

    fn resolve_click(
        &mut self,
        rows: &[Row],
        idx: usize,
        on_chevron: bool,
        now: Instant,
    ) -> NavEffect {
        // A chevron click on a dim directory still toggles it: collapsing and
        // expanding is how the tree is read, and a narrowing must not freeze
        // its shape. Any other click on a dim row is inert (ADR 0023).
        if !rows[idx].live && !(on_chevron && rows[idx].kind.is_dir() && !rows[idx].missing) {
            self.click_timer.disarm();
            return NavEffect::None;
        }
        if on_chevron {
            self.click_timer.disarm();
            self.activate(rows, Some((idx, true)))
        } else if self.click_timer.observe(&rows[idx].path, now) {
            self.activate(rows, Some((idx, false)))
        } else {
            self.view.on_single_click(rows, idx);
            NavEffect::None
        }
    }

    /// Enter / activating-click resolution (clicks arrive here only as
    /// chevron clicks or completed double-clicks — ADR 0015). In picker mode
    /// Enter picks whatever is selected — file or dir; a double-click picks
    /// files but browses dirs (chevrons browse too), so exploratory clicks
    /// never confirm by accident. With a narrowing active there may be no
    /// selection at all, and then there is nothing to pick (ADR 0023).
    fn activate(&mut self, rows: &[Row], click: Option<(usize, bool)>) -> NavEffect {
        let idx = match click {
            Some((idx, _)) => Some(idx),
            None => self.view.sync(rows),
        };
        let Some(idx) = idx else {
            return NavEffect::Message(self.nothing_selectable_message());
        };
        if self.mode.is_pick()
            && let Some(row) = rows.get(idx)
            && !row.missing
        {
            let browsing_click = click.is_some() && row.kind.is_dir();
            if !browsing_click && !row.pickable {
                // Navigable but not confirmable: a directory under a
                // file-shaped filter (task 027).
                return NavEffect::Message(format!("{} does not match the filter", row.name));
            }
            if !browsing_click {
                if click.is_some() {
                    // A picking click also moves the selection there.
                    self.view.focus(row.path.clone());
                }
                self.picked = Some(row.path.clone());
                return NavEffect::None;
            }
        }
        match click {
            Some((idx, on_chevron)) => self.view.on_click(&mut self.tree, rows, idx, on_chevron),
            None => self.view.on_enter(&mut self.tree, rows),
        }
    }

    /// Why nothing happened when there is no selection to act on.
    fn nothing_selectable_message(&self) -> String {
        match &self.search {
            Some(state) if !state.matches.is_empty() => String::new(),
            Some(_) => "no matches".into(),
            None => String::new(),
        }
    }

    // ---- search ----

    fn request_index(&mut self) {
        if !self.index_requested {
            self.index_requested = true;
            self.send_index_rebuild();
        }
    }

    fn send_index_rebuild(&mut self) {
        let _ = self.index_cmds.send(IndexCmd::Rebuild {
            root: self.root.clone(),
            show_hidden: self.settings.show_hidden,
        });
    }

    fn search_push(&mut self, c: char) {
        self.request_index();
        if self.search.is_none() {
            self.search = Some(SearchState {
                query: String::new(),
                matches: Vec::new(),
                matched_set: HashMap::new(),
                saved_selection: self.view.selection.clone(),
                saved_scroll: self.view.scroll,
            });
        }
        if let Some(state) = &mut self.search {
            state.query.push(c);
        }
        self.rematch();
    }

    fn search_pop(&mut self) {
        let Some(state) = &mut self.search else {
            return;
        };
        state.query.pop();
        if state.query.is_empty() {
            // Backspace-to-empty ends the search in place; a reveal from the
            // abandoned query must not keep mutating the tree.
            self.search = None;
            self.pending_reveal = None;
            return;
        }
        self.rematch();
    }

    /// Esc backs out one layer (ADR 0012): an active search clears (tree
    /// mode restores the pre-search view); with nothing to dismiss, Esc
    /// quits — the picker without a pick, the tree like Ctrl-C.
    fn on_esc(&mut self) -> bool {
        match self.search.take() {
            Some(state) => {
                // Both modes restore the pre-search view now: the picker keeps
                // the tree, so it has a view worth putting back (ADR 0023).
                self.view.selection = state.saved_selection;
                self.view.scroll = state.saved_scroll;
                self.pending_reveal = None;
                false
            }
            None => {
                self.pending_reveal = None;
                true
            }
        }
    }

    /// Recomputes matches for the current query. Matches are held in tree
    /// order (ADR 0023 / task 063), and the selection anchors *forward*: it
    /// lands on the first match at or after the current selection, wrapping to
    /// the first match when none follows. A selected row that still matches
    /// therefore never moves, and narrowing a query never drags the pane
    /// backwards.
    fn rematch(&mut self) {
        let Some(query) = self.search.as_ref().map(|s| s.query.clone()) else {
            return;
        };
        let Some(index) = self.index.clone() else {
            if let Some(state) = &mut self.search {
                state.matches = Vec::new();
                state.matched_set = HashMap::new();
            }
            return;
        };
        let mut matches = search(&index, &query);
        if let Some(filter) = &self.filter {
            // The filter defines the corpus, the query ranks what is left
            // (ADR 0023). Directories are never judged by the filter, so a
            // query can still reach them; a filtered-out file can never
            // surface. Filtering the results, not the index, keeps this from
            // forcing a rebuild.
            matches.retain(|m| {
                // Directories are never dimmed by a filter, so a query may
                // still reach them; a filtered-out file can never surface.
                m.entry.is_dir || filter.matches(&m.entry.rel, &m.entry.name, false)
            });
        }
        search::sort_tree_order(&mut matches);
        let matched_set = matches
            .iter()
            .map(|m| {
                let indices = if m.by_path {
                    Vec::new()
                } else {
                    m.indices.clone()
                };
                (m.entry.abs.clone(), indices)
            })
            .collect();
        let current = self.anchor_index(&matches);
        let target = matches.get(current).map(|m| m.entry.abs.clone());
        if let Some(state) = &mut self.search {
            state.matches = matches;
            state.matched_set = matched_set;
        }
        // Reveal the anchored match — but only when it is somewhere new, and
        // never over a reveal already in flight. A rematch also runs on every
        // index refresh: revealing a match the selection already sits on would
        // undo whatever the wheel just did, and overwriting a pending reveal
        // would swallow the keystroke that started it.
        if let Some(target) = target
            && self.pending_reveal.is_none()
            && self.view.selection.as_deref() != Some(target.as_path())
        {
            self.reveal(target);
        }
    }

    /// The forward anchor: the index of the first match at or after the current
    /// selection in tree order, wrapping to the first match when none follows.
    /// Falls back to the first match when there is no selection to anchor on.
    fn anchor_index(&self, matches: &[Match]) -> usize {
        if matches.is_empty() {
            return 0;
        }
        let Some(selection) = &self.view.selection else {
            return 0;
        };
        let Ok(rel) = selection.strip_prefix(&self.root) else {
            return 0;
        };
        let rel = rel.to_string_lossy().replace('\\', "/");
        if rel.is_empty() {
            return 0; // the root row precedes every match
        }
        let is_dir = self
            .tree
            .node_at(selection)
            .is_some_and(|node| node.kind.is_dir());
        let position = search::tree_order_position(matches, &rel, is_dir);
        // Past the last match, the anchor wraps to the first.
        if position == matches.len() {
            0
        } else {
            position
        }
    }

    /// Steps to the next or previous match **relative to the selection**, not
    /// to a remembered pointer. `→`, `←`, and the mouse all move the selection
    /// without going through here, so a stored index goes stale the moment one
    /// of them is used — stepping would then swallow a keystroke or jump
    /// backwards. Deriving the position each time cannot desynchronise.
    fn cycle_match(&mut self, forward: bool) {
        let matches = match &self.search {
            Some(state) if !state.matches.is_empty() => state.matches.clone(),
            _ => return,
        };
        let n = matches.len();
        // The first match at or after the selection, and whether that *is* the
        // selected row.
        let anchor = self.anchor_index(&matches);
        let on_match = self
            .view
            .selection
            .as_deref()
            .is_some_and(|selected| selected == matches[anchor].entry.abs);
        let next = if forward {
            // Off a match, the anchor already sits after it.
            if on_match { (anchor + 1) % n } else { anchor }
        } else {
            (anchor + n - 1) % n
        };
        self.reveal(matches[next].entry.abs.clone());
    }

    /// Expand ancestors toward `path` (requesting loads as needed) and focus
    /// it once visible. Converges over delta round-trips via pending_reveal.
    fn reveal(&mut self, path: PathBuf) {
        self.pending_reveal = Some(path);
        self.step_reveal();
    }

    fn step_reveal(&mut self) {
        let Some(target) = self.pending_reveal.clone() else {
            return;
        };
        let Ok(rel) = target.strip_prefix(&self.root) else {
            self.pending_reveal = None;
            return;
        };
        if rel.as_os_str().is_empty() {
            // The target is the root row itself.
            self.view.focus(target);
            self.pending_reveal = None;
            return;
        }
        // The root row is collapsible; revealing anything below re-opens it.
        if !self.tree.get(self.tree.root()).expanded {
            let root = self.root.clone();
            self.tree.set_expanded(&root, true);
        }
        let mut current = self.root.clone();
        for component in rel.iter() {
            let next = current.join(component);
            match self.tree.node_at(&next) {
                None => {
                    // Snapshots are complete listings: a loaded parent that
                    // lacks the entry means the target is gone (stale index)
                    // — drop the reveal instead of fighting the tree forever.
                    if self.tree.node_at(&current).is_some_and(|n| n.is_loaded()) {
                        self.pending_reveal = None;
                    }
                    return; // otherwise the listing is still in flight
                }
                Some(_) if next == target => {
                    self.view.focus(target.clone());
                    self.pending_reveal = None;
                    return;
                }
                Some(node) => {
                    if !node.kind.is_dir() {
                        self.pending_reveal = None; // path went through a file
                        return;
                    }
                    if !node.expanded {
                        self.tree.set_expanded(&next, true);
                        self.expansion_dirty = true;
                    }
                    if !self.tree.node_at(&next).is_some_and(|n| n.is_loaded()) {
                        // Deduplicated like peeks; the snapshot's arrival
                        // clears the marker.
                        if !self.requested_peeks.contains(&next) {
                            self.requested_peeks.insert(next.clone());
                            let _ = self.source_cmds.send(SourceCmd::Expand(next));
                        }
                        return; // wait for the listing
                    }
                }
            }
            current = next;
        }
        self.pending_reveal = None;
    }

    // ---- rows & drawing ----

    /// The rows, in both modes alike (ADR 0023): the picker renders the same
    /// tree the pane does, with the same narrowing applied — there is no flat
    /// match list.
    fn rows(&self) -> Vec<Row> {
        let git = if self.settings.git {
            self.git_state.as_deref()
        } else {
            None
        };
        let matched = self.search.as_ref().map(|s| &s.matched_set);
        flat_view::visible_rows(
            &self.tree,
            &self.settings,
            Decor {
                git,
                matched,
                home: self.home.as_deref(),
                split: Some(&self.view.split),
                filter: self.filter.as_ref(),
            },
        )
    }

    /// End of every iteration: advance any pending reveal, recompute rows,
    /// reconcile watches, scroll-reconcile, request peek-loads (ADR 0007),
    /// persist expansion changes, draw.
    fn finish_iteration(&mut self, terminal: &mut term::Term) -> io::Result<()> {
        self.process_restores();
        self.step_reveal();
        let rows = self.rows();
        self.rows_len = rows.len();
        let viewport = render::tree_viewport_height(area(terminal));
        if !self.mode.is_pick() {
            self.reconcile_watches(&rows);
        }
        self.view.reconcile(&rows, viewport);
        self.request_peeks(&rows, viewport);
        if self.mode == Mode::Tree {
            self.save_persisted(false);
        }
        let bottom = self.bottom_line();
        let theme = Theme::for_id(self.settings.theme);
        let (view, settings) = (&self.view, &self.settings);
        terminal.draw(|frame| render::draw(frame, &rows, view, settings, &theme, &bottom))?;
        Ok(())
    }

    fn bottom_line(&self) -> String {
        let base = if let Some(state) = &self.search {
            let n = state.matches.len();
            // Same counter in both modes; only the prompt marker differs, so a
            // picker still reads as a picker (ADR 0023).
            let prompt = match self.mode {
                Mode::Tree => format!("search: {}", state.query),
                Mode::Pick => format!("> {}", state.query),
            };
            if n == 0 {
                format!("{prompt} (no matches)")
            } else {
                format!("{prompt} ({}/{n})", self.anchor_index(&state.matches) + 1)
            }
        } else if self.mode.is_pick() {
            "> type to filter, Enter picks the selection, Esc quits".into()
        } else {
            // The root row carries the path annotation; the idle bottom line
            // stays clear for messages.
            String::new()
        };
        if self.status.is_empty() {
            base
        } else {
            format!("{base} — {}", self.status)
        }
    }

    // ---- persistence ----

    fn load_persisted(&mut self) {
        let Some(state) = persist::load(&self.root) else {
            return;
        };
        self.restore_expanded = state
            .expanded
            .iter()
            .map(|rel| self.root.join(rel))
            .collect();
        if let Some(rel) = state.selection {
            self.view.focus(self.root.join(rel));
        }
        self.view.scroll = state.scroll;
        // Seed the saved snapshot (root-relative, like save_persisted
        // compares) so restoring alone doesn't rewrite the file.
        let mut expanded = state.expanded;
        expanded.sort();
        expanded.dedup();
        self.last_saved_expanded = expanded;
    }

    fn save_persisted(&mut self, include_view: bool) {
        if !self.expansion_dirty && !include_view {
            return;
        }
        self.expansion_dirty = false;
        let mut expanded: Vec<PathBuf> = self
            .tree
            .expanded_dirs()
            .into_iter()
            .filter_map(|p| p.strip_prefix(&self.root).ok().map(PathBuf::from))
            .filter(|p| !p.as_os_str().is_empty())
            .collect();
        // Dirs still awaiting restore stay persisted.
        expanded.extend(
            self.restore_expanded
                .iter()
                .filter_map(|p| p.strip_prefix(&self.root).ok().map(PathBuf::from)),
        );
        expanded.sort();
        expanded.dedup();
        if !include_view && expanded == self.last_saved_expanded {
            return;
        }
        self.last_saved_expanded = expanded.clone();
        let state = persist::PersistedState {
            version: persist::VERSION,
            expanded,
            selection: self
                .view
                .selection
                .as_ref()
                .and_then(|p| p.strip_prefix(&self.root).ok().map(PathBuf::from)),
            scroll: self.view.scroll,
        };
        let _ = persist::save(&self.root, &state);
    }

    // ---- watches, peeks, git ----

    /// Watch the root, every expanded dir, and every chain member (a chain
    /// label must update when an intermediate gains a sibling) — but never
    /// ignored dirs. Plus the repo's .git dir for git-state changes.
    fn reconcile_watches(&mut self, rows: &[Row]) {
        let mut desired: HashSet<PathBuf> = HashSet::new();
        desired.insert(self.root.clone());
        if let Some(repo) = &self.repo_root {
            // .git itself (index, HEAD, lock files) plus the refs dirs, so
            // branch updates that touch only nested paths still refresh.
            desired.insert(repo.join(".git"));
            desired.insert(repo.join(".git/refs"));
            desired.insert(repo.join(".git/refs/heads"));
        }
        for row in rows {
            if !row.kind.is_dir() || row.missing || row.ignored {
                continue;
            }
            if !row.chain.is_empty() {
                desired.extend(row.chain.iter().cloned());
            } else if row.expanded {
                desired.insert(row.path.clone());
            }
        }
        for gone in self.watched.difference(&desired) {
            let _ = self.watch_cmds.send(WatchCmd::Unwatch(gone.clone()));
        }
        for new in desired.difference(&self.watched) {
            let _ = self.watch_cmds.send(WatchCmd::Watch(new.clone()));
        }
        self.watched = desired;
    }

    /// One-level loads for unloaded dirs in the viewport, so chains can form
    /// for collapsed dirs too. Bounded by the viewport, deduplicated, never
    /// ignored dirs (which is why peeks wait for the first git answer when a
    /// repo exists), and never through symlinks — only real dirs can join
    /// chains, so only real dirs are worth peeking.
    fn request_peeks(&mut self, rows: &[Row], viewport: usize) {
        if !self.settings.compact {
            return;
        }
        if self.settings.git && self.repo_root.is_some() && !self.git_answered {
            return;
        }
        for row in rows.iter().skip(self.view.scroll).take(viewport) {
            if row.kind == NodeKind::Dir
                && !row.loaded
                && !row.missing
                && !row.ignored
                && !self.requested_peeks.contains(&row.path)
            {
                self.requested_peeks.insert(row.path.clone());
                let _ = self.source_cmds.send(SourceCmd::Expand(row.path.clone()));
            }
        }
    }

    /// Scrolls by `delta` rows against the count from the last draw. Kept
    /// separate from the rows so a wheel burst costs a clamp, not a tree walk.
    fn scroll_rows(&mut self, delta: isize, viewport: usize) {
        let max_scroll = self.rows_len.saturating_sub(viewport);
        self.view.scroll = self
            .view
            .scroll
            .saturating_add_signed(delta)
            .min(max_scroll);
    }

    fn refresh_git(&mut self) {
        if self.settings.git
            && let Some(repo) = &self.repo_root
        {
            let _ = self.git_cmds.send(GitCmd::Refresh { repo: repo.clone() });
        }
    }

    // ---- opening ----

    /// Fire-and-forget open (GUI dispatchers); a background thread reaps the
    /// child so it never zombifies.
    fn open_detached(&mut self, path: &Path) {
        let argv = self.open_cmd.build(path);
        let (program, args) = argv.split_first().expect("argv is never empty");
        let spawned = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match spawned {
            Ok(mut child) => {
                thread::spawn(move || {
                    let _ = child.wait();
                });
            }
            Err(e) => self.status = format!("open failed: {program}: {e}"),
        }
    }

    /// Hands the terminal to the child and waits. The input thread polls in
    /// 100 ms slices and checks the pause flag between slices, so after
    /// setting the flag this waits one slice for the thread to park before
    /// the child starts reading the tty.
    fn open_in_terminal(&mut self, terminal: &mut term::Term, path: &Path) {
        let argv = self.open_cmd.build(path);
        let (program, args) = argv.split_first().expect("argv is never empty");
        self.input_paused.store(true, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(120));
        term::restore(self.settings.mouse, self.mode.is_pick());
        self.yielded_terminal = true;
        let result = Command::new(program).args(args).status();
        let reentered = term::reenter(self.settings.mouse, self.mode.is_pick());
        self.input_paused.store(false, Ordering::SeqCst);
        let _ = terminal.clear();
        match result {
            Ok(code) if !code.success() => {
                self.status = format!("open: {program} exited with {code}");
            }
            Ok(_) => {}
            Err(e) => self.status = format!("open failed: {program}: {e}"),
        }
        if let Err(e) = reentered {
            self.status = format!("terminal re-init failed: {e}");
        }
    }
}

/// What a ctl verb defers until after its reply is sent.
enum CtlEffect {
    None,
    Open(PathBuf),
    Quit,
}

/// Resolves `.` and `..` segments lexically, touching no filesystem.
fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// Canonicalizes the longest existing prefix of `path` and re-appends any
/// not-yet-existing tail, so a path whose leaf does not exist yet still has its
/// symlinked ancestors resolved. Falls back to the input if nothing resolves.
fn canonicalize_lenient(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    match (path.parent(), path.file_name()) {
        (Some(parent), Some(name)) => canonicalize_lenient(parent).join(name),
        _ => path.to_path_buf(),
    }
}

/// Resolves a `reveal` argument to a path inside `root` (canonicalized at
/// launch), or `None` if it lies outside. The path is taken **as given** first
/// — lexically normalized (so `..` cannot escape) and matched against the root,
/// which keeps relative inputs and in-tree symlink nodes working with no
/// resolution — and symlinks are resolved only as a *fallback* for a path that
/// did not already match (e.g. a symlinked root prefix such as macOS `/tmp` →
/// `/private/tmp`). The lexical path is revealed when it matches; otherwise the
/// resolved path (which is under the canonical root) is.
fn resolve_within_root(root: &Path, input: &Path) -> Option<PathBuf> {
    let abs = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    let abs = lexical_normalize(&abs);
    if abs.starts_with(root) {
        return Some(abs);
    }
    let resolved = canonicalize_lenient(&abs);
    resolved.starts_with(root).then_some(resolved)
}

fn area(terminal: &term::Term) -> Rect {
    terminal
        .size()
        .map(|s| Rect::new(0, 0, s.width, s.height))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use birch_core::git::parse_porcelain_v2;
    use birch_core::search::SearchIndex;
    use birch_core::{Entry, NodeKind};

    use super::*;

    pub(super) struct Harness {
        pub(super) app: App,
        pub(super) source_rx: mpsc::Receiver<SourceCmd>,
        _watch_rx: mpsc::Receiver<WatchCmd>,
        _index_rx: mpsc::Receiver<IndexCmd>,
        _git_rx: mpsc::Receiver<GitCmd>,
    }

    pub(super) fn harness(mode: Mode) -> Harness {
        let (source_tx, source_rx) = mpsc::channel();
        let (watch_tx, watch_rx) = mpsc::channel();
        let (index_tx, index_rx) = mpsc::channel();
        let (git_tx, git_rx) = mpsc::channel();
        let root = PathBuf::from("/r");
        let mut app = App {
            tree: Tree::new(root.clone()),
            view: FlatView::default(),
            settings: Settings::default(),
            open_cmd: OpenCmd::from_template("editor {}").expect("static template"),
            mode,
            status: String::new(),
            root_label: "r".into(),
            root,
            source_cmds: source_tx,
            watch_cmds: watch_tx,
            index_cmds: index_tx,
            git_cmds: git_tx,
            repo_root: None,
            socket: None,
            filter: None,
            git_state: None,
            git_answered: false,
            watched: HashSet::new(),
            requested_peeks: HashSet::new(),
            index: None,
            index_requested: false,
            search: None,
            pending_reveal: None,
            home: None,
            restore_expanded: HashSet::new(),
            expansion_dirty: false,
            last_saved_expanded: Vec::new(),
            picked: None,
            input_paused: Arc::new(AtomicBool::new(false)),
            click_timer: input::ClickTimer::default(),
            armed_press: None,
            rows_len: 0,
            yielded_terminal: false,
        };
        app.tree.set_expanded(Path::new("/r"), true);
        Harness {
            app,
            source_rx,
            _watch_rx: watch_rx,
            _index_rx: index_rx,
            _git_rx: git_rx,
        }
    }

    pub(super) fn feed(app: &mut App, dir: &str, entries: &[(&str, NodeKind)]) {
        let entries = entries
            .iter()
            .map(|(name, kind)| Entry {
                name: (*name).into(),
                kind: *kind,
            })
            .collect();
        app.handle_source(SourceEvent::Deltas(vec![TreeDelta::Snapshot {
            dir: dir.into(),
            entries,
        }]));
    }

    pub(super) fn drain_expands(rx: &mpsc::Receiver<SourceCmd>) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        while let Ok(SourceCmd::Expand(dir)) = rx.try_recv() {
            dirs.push(dir);
        }
        dirs
    }

    fn index_of(entries: &[(&str, bool)]) -> Arc<SearchIndex> {
        Arc::new(SearchIndex {
            entries: entries
                .iter()
                .map(|(rel, is_dir)| {
                    birch_core::search::IndexEntry::new(
                        (*rel).into(),
                        PathBuf::from("/r").join(rel),
                        *is_dir,
                    )
                })
                .collect(),
        })
    }

    #[test]
    fn reveal_cascades_expands_and_converges() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a", NodeKind::Dir)]);
        drain_expands(&h.source_rx);

        h.app.reveal("/r/a/b/c.txt".into());
        assert_eq!(drain_expands(&h.source_rx), [PathBuf::from("/r/a")]);
        assert!(h.app.tree.node_at(Path::new("/r/a")).unwrap().expanded);
        // Repeated steps while waiting do not duplicate the load request.
        h.app.step_reveal();
        assert!(drain_expands(&h.source_rx).is_empty());

        feed(&mut h.app, "/r/a", &[("b", NodeKind::Dir)]);
        h.app.step_reveal();
        assert_eq!(drain_expands(&h.source_rx), [PathBuf::from("/r/a/b")]);
        feed(&mut h.app, "/r/a/b", &[("c.txt", NodeKind::File)]);
        h.app.step_reveal();
        assert!(h.app.pending_reveal.is_none());
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a/b/c.txt"))
        );
    }

    #[test]
    fn stale_reveal_is_dropped_not_looped() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("real.txt", NodeKind::File)]);
        h.app.reveal("/r/ghost.txt".into());
        assert!(
            h.app.pending_reveal.is_none(),
            "loaded parent without the entry drops it"
        );

        // A reveal through a file component is equally dead.
        h.app.reveal("/r/real.txt/inner".into());
        assert!(h.app.pending_reveal.is_none());
    }

    #[test]
    fn search_type_backspace_esc_transitions() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("zzz.txt", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("src/main.rs", false), ("zzz.txt", false)]));
        h.app.view.focus("/r/zzz.txt".into());
        drain_expands(&h.source_rx);

        h.app.search_push('m');
        let state = h.app.search.as_ref().expect("search active");
        assert_eq!(state.query, "m");
        assert!(state.matched_set.contains_key(Path::new("/r/src/main.rs")));
        // The jump revealed toward the best match (expanding src).
        assert!(h.app.tree.node_at(Path::new("/r/src")).unwrap().expanded);

        // Backspace to empty ends the search and cancels the reveal.
        h.app.search_pop();
        assert!(h.app.search.is_none());
        assert!(h.app.pending_reveal.is_none());

        // Esc restores the pre-search view.
        h.app.view.focus("/r/zzz.txt".into());
        h.app.search_push('m');
        assert!(!h.app.on_esc());
        assert!(h.app.search.is_none());
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/zzz.txt"))
        );
    }

    #[test]
    fn cycle_wraps_and_survives_an_index_refresh() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a.txt", NodeKind::File), ("ab.txt", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("a.txt", false), ("ab.txt", false)]));
        h.app.search_push('a');
        assert_eq!(h.app.search.as_ref().unwrap().matches.len(), 2);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/a.txt")));

        // Stepping is expressed by where the selection lands, which is the only
        // thing the user can see — and the only thing that cannot go stale.
        h.app.cycle_match(true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/ab.txt"))
        );
        h.app.cycle_match(true);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/a.txt")));
        h.app.cycle_match(false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/ab.txt"))
        );

        // A fresh index (watcher churn) leaves the cycled position alone.
        h.app.index = Some(index_of(&[("a.txt", false), ("ab.txt", false)]));
        h.app.rematch();
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/ab.txt"))
        );
    }

    #[test]
    fn stepping_and_anchoring_follow_tree_order() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("src", NodeKind::Dir),
                ("mid.md", NodeKind::File),
                ("zed.md", NodeKind::File),
            ],
        );
        feed(&mut h.app, "/r/src", &[("deep.md", NodeKind::File)]);
        // Index order is deliberately not tree order — cycling must follow the
        // rows, not the index (and not the fuzzy score).
        h.app.index = Some(index_of(&[
            ("zed.md", false),
            ("src/deep.md", false),
            ("mid.md", false),
        ]));

        // Anchor from the root row: the first match in tree order.
        h.app.view.focus("/r".into());
        h.app.search_push('d');
        let tree_order: Vec<PathBuf> = h
            .app
            .search
            .as_ref()
            .unwrap()
            .matches
            .iter()
            .map(|m| m.entry.abs.clone())
            .collect();
        assert_eq!(
            tree_order,
            [
                PathBuf::from("/r/src/deep.md"),
                PathBuf::from("/r/mid.md"),
                PathBuf::from("/r/zed.md"),
            ],
            "directories sort first, then files by name"
        );
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/src/deep.md")),
            "the anchor starts on the first match in tree order"
        );

        // Stepping walks down the tree, then wraps.
        h.app.cycle_match(true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/mid.md"))
        );
        h.app.cycle_match(true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/zed.md"))
        );
        h.app.cycle_match(true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/src/deep.md"))
        );
    }

    #[test]
    fn search_anchors_forward_from_the_selection() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("ad.md", NodeKind::File),
                ("bd.md", NodeKind::File),
                ("cd.md", NodeKind::File),
            ],
        );
        h.app.index = Some(index_of(&[
            ("ad.md", false),
            ("bd.md", false),
            ("cd.md", false),
        ]));

        // Sitting on a row between matches: the search moves forward, never back.
        h.app.view.focus("/r/bd.md".into());
        h.app.search_push('d');
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/bd.md")),
            "the selected row matches, so it stays"
        );

        // Narrowing to a query the selection fails moves to the next match.
        h.app.search_push('c'); // "dc" matches nothing
        h.app.search_pop();
        h.app.index = Some(index_of(&[("ad.md", false), ("cd.md", false)]));
        h.app.rematch();
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/cd.md")),
            "the anchor moves forward to the next match, not back to the first"
        );

        // Past the last match, the anchor wraps to the first.
        h.app.view.focus("/r/zz.md".into());
        h.app.rematch();
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/ad.md")));
    }

    #[test]
    fn restores_cascade_and_skip_ignored_and_stale() {
        let mut h = harness(Mode::Tree);
        h.app.repo_root = Some("/r".into());
        h.app.git_answered = true;
        let git = parse_porcelain_v2(b"! ign/\0", Path::new("/r"));
        h.app.git_state = Some(Arc::new(git));
        h.app.restore_expanded = ["/r/keep", "/r/ign", "/r/gone"]
            .iter()
            .map(PathBuf::from)
            .collect();

        feed(
            &mut h.app,
            "/r",
            &[("keep", NodeKind::Dir), ("ign", NodeKind::Dir)],
        );
        drain_expands(&h.source_rx);
        h.app.process_restores();

        assert!(h.app.tree.node_at(Path::new("/r/keep")).unwrap().expanded);
        assert!(!h.app.tree.node_at(Path::new("/r/ign")).unwrap().expanded);
        assert!(h.app.restore_expanded.is_empty(), "stale /r/gone dropped");
        assert_eq!(drain_expands(&h.source_rx), [PathBuf::from("/r/keep")]);
    }

    #[test]
    fn restores_wait_for_the_first_git_answer() {
        let mut h = harness(Mode::Tree);
        h.app.repo_root = Some("/r".into());
        h.app.restore_expanded = [PathBuf::from("/r/dir")].into_iter().collect();
        feed(&mut h.app, "/r", &[("dir", NodeKind::Dir)]);
        h.app.process_restores();
        assert!(!h.app.tree.node_at(Path::new("/r/dir")).unwrap().expanded);
        h.app.git_answered = true;
        h.app.process_restores();
        assert!(h.app.tree.node_at(Path::new("/r/dir")).unwrap().expanded);
    }

    #[test]
    fn reveal_reopens_a_collapsed_root() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        feed(&mut h.app, "/r/src", &[("main.rs", NodeKind::File)]);
        h.app.tree.set_expanded(Path::new("/r"), false);
        drain_expands(&h.source_rx);

        h.app.reveal("/r/src/main.rs".into());
        assert!(h.app.tree.get(h.app.tree.root()).expanded);
        h.app.step_reveal();
        assert!(h.app.pending_reveal.is_none());
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/src/main.rs"))
        );

        // Revealing the root itself focuses its row.
        h.app.reveal("/r".into());
        assert!(h.app.pending_reveal.is_none());
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r")));
    }

    #[test]
    fn esc_backs_out_search_then_quits() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        h.app.index = Some(index_of(&[("a.txt", false)]));
        h.app.search_push('a');
        assert!(!h.app.on_esc(), "first Esc clears the search");
        assert!(h.app.search.is_none());
        assert!(h.app.on_esc(), "second Esc quits (ADR 0012)");
    }

    #[test]
    fn pick_enter_on_root_picks_the_root() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("sub", NodeKind::Dir)]);
        let rows = h.app.rows();
        assert_eq!(rows[0].path, Path::new("/r"));
        // A fresh session: sync lands on row 0, and Enter picks the root —
        // the explicit "this dir" answer.
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r")));
    }

    #[test]
    fn picker_enter_picks_anything_clicks_browse_dirs() {
        let mut h = harness(Mode::Pick);
        feed(
            &mut h.app,
            "/r",
            &[("sub", NodeKind::Dir), ("file.txt", NodeKind::File)],
        );
        // Dir clicks browse — name or chevron — never confirm.
        let rows = h.app.rows();
        let sub_idx = rows.iter().position(|r| r.name == "sub").unwrap();
        h.app.activate(&rows, Some((sub_idx, true)));
        assert!(h.app.picked.is_none(), "chevron click browses");
        assert!(h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded);
        let rows = h.app.rows();
        h.app.activate(&rows, Some((sub_idx, false)));
        assert!(h.app.picked.is_none(), "name click browses (collapses)");
        assert!(!h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded);

        // File clicks pick.
        let rows = h.app.rows();
        let file_idx = rows.iter().position(|r| r.name == "file.txt").unwrap();
        h.app.activate(&rows, Some((file_idx, false)));
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/file.txt")));

        // Enter picks whatever is selected — a dir included.
        h.app.picked = None;
        let rows = h.app.rows();
        h.app.view.focus("/r/sub".into());
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/sub")));
    }

    #[test]
    fn click_selects_double_click_activates() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("sub", NodeKind::Dir), ("file.txt", NodeKind::File)],
        );
        let rows = h.app.rows();
        let file_idx = rows.iter().position(|r| r.name == "file.txt").unwrap();
        let t0 = Instant::now();
        // First click: selection only.
        assert!(matches!(
            h.app.resolve_click(&rows, file_idx, false, t0),
            NavEffect::None
        ));
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/file.txt"))
        );
        // Second click inside the window: opens.
        assert!(matches!(
            h.app.resolve_click(&rows, file_idx, false, t0 + Duration::from_millis(100)),
            NavEffect::Open(p) if p == Path::new("/r/file.txt")
        ));
    }

    #[test]
    fn chevron_click_toggles_and_disarms_the_double() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("sub", NodeKind::Dir)]);
        let rows = h.app.rows();
        let sub_idx = rows.iter().position(|r| r.name == "sub").unwrap();
        let t0 = Instant::now();
        // Arm on the name, toggle via the chevron, then a fast name click:
        // it must select, not complete a double.
        h.app.resolve_click(&rows, sub_idx, false, t0);
        h.app
            .resolve_click(&rows, sub_idx, true, t0 + Duration::from_millis(50));
        assert!(h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded);
        let rows = h.app.rows();
        let sub_idx = rows.iter().position(|r| r.name == "sub").unwrap();
        assert!(matches!(
            h.app
                .resolve_click(&rows, sub_idx, false, t0 + Duration::from_millis(100)),
            NavEffect::None
        ));
        assert!(
            h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded,
            "single name click after a chevron toggle must not re-toggle"
        );
    }

    #[test]
    fn picker_chevron_click_browses_and_never_picks() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        h.app.index = Some(index_of(&[("src", true)]));
        h.app.search_push('s');
        let rows = h.app.rows();
        assert_eq!(rows[1].name, "src");
        let t0 = Instant::now();
        // The picker renders the real tree now (ADR 0023), so a chevron click
        // toggles the directory. It must still never confirm the pick.
        assert!(matches!(
            h.app.resolve_click(&rows, 1, true, t0),
            NavEffect::None | NavEffect::RequestExpand(_)
        ));
        assert!(h.app.picked.is_none());
        assert!(h.app.tree.node_at(Path::new("/r/src")).unwrap().expanded);
        // A completed double-click on a directory browses it too — only Enter
        // confirms a directory, so exploratory clicks never pick by accident.
        h.app
            .resolve_click(&rows, 1, false, t0 + Duration::from_millis(100));
        assert!(h.app.picked.is_none());
        // Enter on the same row picks it.
        let rows = h.app.rows();
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/src")));
    }

    #[test]
    fn picker_search_keeps_the_tree_and_dims_non_matches() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        feed(&mut h.app, "/r/src", &[("main.rs", NodeKind::File)]);
        h.app.tree.set_expanded(Path::new("/r/src"), true);
        h.app.index = Some(index_of(&[("src/main.rs", false), ("src", true)]));
        h.app.search_push('m');

        // The tree survives the query: root and ancestors still render, at
        // their real depths, instead of a flat list of hits (ADR 0023).
        let rows = h.app.rows();
        let names: Vec<(&str, usize, bool)> = rows
            .iter()
            .map(|r| (r.name.as_str(), r.depth, r.live))
            .collect();
        assert_eq!(
            names,
            [("r", 0, false), ("src", 1, false), ("main.rs", 2, true)],
            "ancestors stay visible but dim; only the match is live"
        );

        // The selection anchored to the match, and Enter picks it.
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/src/main.rs"))
        );
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/src/main.rs")));
    }

    /// Applies a glob filter to a harness, as `--filter`/`--filter-mode` do.
    fn with_filter(app: &mut App, patterns: &[&str], mode: birch_core::FilterMode) {
        let owned: Vec<String> = patterns.iter().map(|p| (*p).to_string()).collect();
        app.filter = Filter::parse(&owned, mode).expect("patterns compile");
    }

    #[test]
    fn filter_skip_dims_files_but_keeps_folders_navigable() {
        use birch_core::FilterMode;
        let mut h = harness(Mode::Pick);
        with_filter(&mut h.app, &["*.md"], FilterMode::Skip);
        feed(
            &mut h.app,
            "/r",
            &[
                ("src", NodeKind::Dir),
                ("notes.md", NodeKind::File),
                ("build.rs", NodeKind::File),
            ],
        );
        let rows = h.app.rows();
        let by_name = |name: &str| {
            rows.iter()
                .find(|r| r.name == name)
                .unwrap_or_else(|| panic!("{name} is present"))
        };

        // A matching file: live and pickable.
        assert!(by_name("notes.md").live && by_name("notes.md").pickable);
        // A non-matching file: shown, dimmed, inert.
        assert!(!by_name("build.rs").live);
        assert!(!by_name("build.rs").pickable);
        // A directory: never dimmed by a file-shaped filter, so the tree stays
        // navigable — but it cannot be picked, since it does not match.
        assert!(by_name("src").live, "folders stay navigable");
        assert!(!by_name("src").pickable, "but only match-able folders pick");

        // Enter on the folder reports instead of picking.
        h.app.view.focus("/r/src".into());
        let effect = h.app.activate(&rows, None);
        assert_eq!(
            effect,
            NavEffect::Message("src does not match the filter".into())
        );
        assert!(h.app.picked.is_none());

        // Enter on the matching file picks it.
        h.app.view.focus("/r/notes.md".into());
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/notes.md")));
    }

    #[test]
    fn filtered_out_files_are_inert_to_keyboard_and_mouse() {
        // 027 asks for this explicitly; it was only ever covered under a
        // *search* before, never under a filter.
        use birch_core::FilterMode;
        let mut h = harness(Mode::Pick);
        with_filter(&mut h.app, &["*.md"], FilterMode::Skip);
        feed(
            &mut h.app,
            "/r",
            &[
                ("a.md", NodeKind::File),
                ("b.rs", NodeKind::File),
                ("c.md", NodeKind::File),
            ],
        );
        h.app.view.focus("/r/a.md".into());
        let rows = h.app.rows();
        let dim = rows.iter().position(|r| r.name == "b.rs").unwrap();
        assert!(!rows[dim].live);

        // The keyboard steps over it.
        h.app.view.move_by(&rows, 1);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/c.md")));

        // The mouse cannot land on it, and it cannot be picked.
        h.app.resolve_click(&rows, dim, false, Instant::now());
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/c.md")));
        assert!(h.app.picked.is_none());
    }

    #[test]
    fn filter_hide_omits_files_but_never_folders() {
        use birch_core::FilterMode;
        let mut h = harness(Mode::Tree);
        with_filter(&mut h.app, &["*.md"], FilterMode::Hide);
        feed(
            &mut h.app,
            "/r",
            &[
                ("docs", NodeKind::Dir),
                ("dead", NodeKind::Dir),
                ("notes.md", NodeKind::File),
                ("build.rs", NodeKind::File),
            ],
        );
        feed(&mut h.app, "/r/docs", &[("guide.md", NodeKind::File)]);
        feed(&mut h.app, "/r/dead", &[("main.rs", NodeKind::File)]);

        let names: Vec<String> = h.app.rows().iter().map(|r| r.name.clone()).collect();
        assert!(names.iter().any(|n| n == "notes.md"));
        assert!(
            !names.iter().any(|n| n == "build.rs"),
            "a non-matching file is omitted in hide mode"
        );
        // Directories stay even when nothing under them matches: the tree
        // loads lazily, so hiding them made rows vanish mid-browse as listings
        // arrived. An empty branch beats a tree that rearranges itself.
        assert!(names.iter().any(|n| n.starts_with("docs")), "{names:?}");
        assert!(names.iter().any(|n| n.starts_with("dead")), "{names:?}");
    }

    #[test]
    fn a_trailing_slash_filters_to_directories() {
        use birch_core::FilterMode;
        let mut h = harness(Mode::Pick);
        with_filter(&mut h.app, &["*/"], FilterMode::Skip);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("notes.md", NodeKind::File)],
        );
        let rows = h.app.rows();
        let row = |name: &str| rows.iter().find(|r| r.name == name).unwrap();

        // `*/` is any directory, so directories become the pickable set...
        assert!(row("src").live && row("src").pickable);
        // ...and every file is dimmed out, since no pattern names a file.
        assert!(!row("notes.md").live);

        h.app.view.focus("/r/src".into());
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/src")));
    }

    #[test]
    fn search_cannot_surface_a_filtered_out_file() {
        use birch_core::FilterMode;
        let mut h = harness(Mode::Pick);
        with_filter(&mut h.app, &["*.md"], FilterMode::Skip);
        feed(
            &mut h.app,
            "/r",
            &[("notes.md", NodeKind::File), ("notes.rs", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("notes.md", false), ("notes.rs", false)]));
        h.app.search_push('n');

        let matched: Vec<String> = h
            .app
            .search
            .as_ref()
            .unwrap()
            .matches
            .iter()
            .map(|m| m.entry.rel.clone())
            .collect();
        assert_eq!(
            matched,
            ["notes.md"],
            "the filter is the corpus; the query ranks what is left"
        );
    }

    #[test]
    fn path_patterns_match_below_the_root() {
        use birch_core::FilterMode;
        let mut h = harness(Mode::Tree);
        with_filter(&mut h.app, &["src/*.rs"], FilterMode::Skip);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        feed(
            &mut h.app,
            "/r/src",
            &[("main.rs", NodeKind::File), ("notes.md", NodeKind::File)],
        );
        h.app.tree.set_expanded(Path::new("/r/src"), true);

        let rows = h.app.rows();
        let live = |name: &str| rows.iter().find(|r| r.name == name).unwrap().live;
        assert!(live("main.rs"), "src/main.rs matches src/*.rs");
        assert!(!live("notes.md"));
    }

    #[test]
    fn scrolling_during_a_search_is_not_snapped_back() {
        let mut h = harness(Mode::Tree);
        let entries: Vec<(String, NodeKind)> = (0..40)
            .map(|i| (format!("f{i:02}.txt"), NodeKind::File))
            .collect();
        let refs: Vec<(&str, NodeKind)> = entries.iter().map(|(n, k)| (n.as_str(), *k)).collect();
        feed(&mut h.app, "/r", &refs);
        h.app.index = Some(index_of(&[("f00.txt", false)]));
        h.app.search_push('f');
        h.app.search_push('0');
        h.app.search_push('0');

        // The match is at the top; the wheel scrolls well past it.
        let rows = h.app.rows();
        let viewport = 10;
        h.app.view.reconcile(&rows, viewport); // consumes the reveal's follow
        h.app.view.scroll_by(&rows, 12, viewport);
        assert_eq!(h.app.view.scroll, 12);

        // Redraws keep coming while the search is live; none of them may drag
        // the viewport back to the selected match.
        for _ in 0..3 {
            h.app.step_reveal();
            let rows = h.app.rows();
            h.app.view.reconcile(&rows, viewport);
        }
        assert_eq!(
            h.app.view.scroll, 12,
            "free scrolling is never snapped back to the selection"
        );
    }

    #[test]
    fn an_index_refresh_does_not_drag_a_scrolled_viewport() {
        // The reported snap-back: with a search open, every index refresh
        // re-ran the match and revealed it, pulling the pane back to the
        // selection however far the wheel had scrolled.
        let mut h = harness(Mode::Tree);
        let entries: Vec<(String, NodeKind)> = (0..40)
            .map(|i| (format!("f{i:02}.txt"), NodeKind::File))
            .collect();
        let refs: Vec<(&str, NodeKind)> = entries.iter().map(|(n, k)| (n.as_str(), *k)).collect();
        feed(&mut h.app, "/r", &refs);
        h.app.index = Some(index_of(&[("f00.txt", false)]));
        h.app.search_push('f');
        h.app.search_push('0');
        h.app.search_push('0');

        let rows = h.app.rows();
        let viewport = 10;
        h.app.view.reconcile(&rows, viewport);
        h.app.view.scroll_by(&rows, 20, viewport);
        let scrolled = h.app.view.scroll;
        assert!(scrolled > 0, "the wheel moved the viewport");

        // A fresh index arrives (watcher churn) while the search is open.
        h.app.index = Some(index_of(&[("f00.txt", false)]));
        h.app.rematch();
        let rows = h.app.rows();
        h.app.view.reconcile(&rows, viewport);
        assert_eq!(
            h.app.view.scroll, scrolled,
            "a refresh that does not move the match must not move the viewport"
        );

        // Typing still takes the pane to the match it selects.
        h.app.view.scroll_by(&rows, 5, viewport);
        h.app.search_pop();
        let rows = h.app.rows();
        h.app.view.reconcile(&rows, viewport);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/f00.txt"))
        );
    }

    #[test]
    fn stepping_follows_the_selection_after_an_arrow_or_a_click() {
        // Independent-review finding: `→`, `←` and clicks move the selection
        // without going through cycle_match, so a remembered match pointer goes
        // stale — `↓` then either swallowed a keystroke or jumped backwards.
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("aa.md", NodeKind::File),
                ("bb.md", NodeKind::File),
                ("cc.md", NodeKind::File),
            ],
        );
        h.app.index = Some(index_of(&[
            ("aa.md", false),
            ("bb.md", false),
            ("cc.md", false),
        ]));
        h.app.search_push('m');
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/aa.md")));

        // `→` advances to the next match; `↓` must then continue from there.
        let rows = h.app.rows();
        h.app.view.on_right(&mut h.app.tree, &rows);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/bb.md")));
        h.app.cycle_match(true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/cc.md")),
            "the keystroke must not be swallowed"
        );

        // A click moves the selection too; stepping continues from the click.
        let rows = h.app.rows();
        let idx = rows.iter().position(|r| r.name == "aa.md").unwrap();
        h.app.resolve_click(&rows, idx, false, Instant::now());
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/aa.md")));
        h.app.cycle_match(true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/bb.md")),
            "stepping must not jump backwards to a stale pointer"
        );
    }

    #[test]
    fn a_dim_folders_chevron_still_toggles() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("keep.md", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("keep.md", false)]));
        h.app.search_push('k');

        let rows = h.app.rows();
        let idx = rows.iter().position(|r| r.name == "src").unwrap();
        assert!(!rows[idx].live, "src does not match the query, so it dims");

        // A chevron click on it expands anyway: structure stays reachable.
        h.app.resolve_click(&rows, idx, true, Instant::now());
        assert!(h.app.tree.node_at(Path::new("/r/src")).unwrap().expanded);
        // ...without stealing the selection, which stays on the match.
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/keep.md"))
        );

        // A name click on the same dim row still does nothing.
        let rows = h.app.rows();
        let idx = rows.iter().position(|r| r.name == "src").unwrap();
        h.app.resolve_click(&rows, idx, false, Instant::now());
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/keep.md"))
        );
    }

    #[test]
    fn right_advances_over_dim_rows_under_a_search() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("aa.md", NodeKind::File),
                ("skip.txt", NodeKind::File),
                ("zz.md", NodeKind::File),
            ],
        );
        h.app.index = Some(index_of(&[("aa.md", false), ("zz.md", false)]));
        h.app.search_push('m');

        // Anchored on the first match; `→` advances to the next *live* row,
        // stepping over the dim one between them (060 + ADR 0023).
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/aa.md")));
        let rows = h.app.rows();
        h.app.view.on_right(&mut h.app.tree, &rows);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/zz.md")));

        // No live row follows, so it stays — the one case where `→` is inert.
        h.app.view.on_right(&mut h.app.tree, &rows);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/zz.md")));
    }

    #[test]
    fn dim_rows_are_inert_and_no_match_means_no_selection() {
        let mut h = harness(Mode::Pick);
        feed(
            &mut h.app,
            "/r",
            &[("keep.md", NodeKind::File), ("other.txt", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("keep.md", false), ("other.txt", false)]));
        h.app.search_push('k');

        let rows = h.app.rows();
        let dim = rows
            .iter()
            .position(|r| r.name == "other.txt")
            .expect("row present");
        assert!(!rows[dim].live);

        // A click on a dim row neither selects nor picks.
        h.app.resolve_click(&rows, dim, false, Instant::now());
        assert!(h.app.picked.is_none());
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/keep.md")),
            "the selection stays on the match"
        );

        // Stepping never lands on a dim row either.
        h.app.view.move_by(&rows, 1);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/keep.md"))
        );

        // A query that matches nothing leaves nothing selected, and Enter
        // picks nothing and says why.
        h.app.search_push('z');
        let rows = h.app.rows();
        assert!(rows.iter().all(|r| !r.live));
        assert!(h.app.view.sync(&rows).is_none());
        let effect = h.app.activate(&rows, None);
        assert_eq!(effect, NavEffect::Message("no matches".into()));
        assert!(h.app.picked.is_none());
        // Independent-review finding: reporting "no selection" is not enough —
        // the field itself must be cleared, or the painter keeps drawing a
        // cursor on a dim row that answers to no key.
        assert!(
            h.app.view.selection.is_none(),
            "no live row means no selection, not a phantom one"
        );
    }

    // ---- ADR 0024: one frame per batch ----

    #[test]
    fn drain_batch_takes_everything_already_queued() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..5 {
            tx.send(AppEvent::Shutdown).unwrap();
        }
        let mut seen = 0;
        let stop = drain_batch(&rx, Instant::now() + Duration::from_secs(5), |_| {
            seen += 1;
            BatchStep::Continue
        });
        assert_eq!(seen, 5);
        assert_eq!(stop, BatchStop::Drained);
    }

    #[test]
    fn drain_batch_never_blocks_on_an_empty_channel() {
        // An idle birch must still draw one frame per event, so a batch that
        // finds nothing queued returns at once rather than waiting out its
        // budget.
        let (_tx, rx) = mpsc::channel::<AppEvent>();
        let started = Instant::now();
        let stop = drain_batch(&rx, started + Duration::from_secs(30), |_| {
            BatchStep::Continue
        });
        assert_eq!(stop, BatchStop::Drained);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn drain_batch_stops_at_the_budget() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..500 {
            tx.send(AppEvent::Shutdown).unwrap();
        }
        let mut seen = 0;
        // A budget already spent draws before consuming anything more.
        let stop = drain_batch(&rx, Instant::now(), |_| {
            seen += 1;
            BatchStep::Continue
        });
        assert_eq!(seen, 0);
        assert_eq!(stop, BatchStop::Budget);
    }

    #[test]
    fn drain_batch_ends_on_handoff_and_on_quit() {
        for (step, expected) in [
            (BatchStep::Stop, BatchStop::Handoff),
            (BatchStep::Quit, BatchStop::Quit),
        ] {
            let (tx, rx) = mpsc::channel();
            for _ in 0..4 {
                tx.send(AppEvent::Shutdown).unwrap();
            }
            let mut seen = 0;
            let stop = drain_batch(&rx, Instant::now() + Duration::from_secs(5), |_| {
                seen += 1;
                step
            });
            // The event that stopped the batch was handled; the rest were not.
            assert_eq!(seen, 1);
            assert_eq!(stop, expected);
        }
    }

    #[test]
    fn scroll_uses_the_cached_row_count_not_a_rebuild() {
        let mut h = harness(Mode::Tree);
        h.app.rows_len = 100;
        h.app.scroll_rows(30, 10);
        assert_eq!(h.app.view.scroll, 30);
        // Clamped to rows_len - viewport, so overscrolling stops rather than
        // running away.
        h.app.scroll_rows(1000, 10);
        assert_eq!(h.app.view.scroll, 90);
        h.app.scroll_rows(-1000, 10);
        assert_eq!(h.app.view.scroll, 0);
    }

    #[test]
    fn scroll_does_nothing_when_the_rows_fit() {
        let mut h = harness(Mode::Tree);
        h.app.rows_len = 4;
        h.app.scroll_rows(30, 10);
        assert_eq!(h.app.view.scroll, 0);
    }

    #[test]
    fn ctl_set_scroll_lines_accepts_the_range_and_refuses_the_rest() {
        let mut h = harness(Mode::Tree);
        let mut req = Request::new(Verb::Set);
        req.setting = Some(SettingKey::ScrollLines);

        req.value = Some("7".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert_eq!(h.app.settings.scroll_lines, 7);

        // Out of range, not a number, and empty are all refused, and none of
        // them disturbs the value already set.
        for bad in ["0", "11", "250", "abc", "", "-1", "3.5"] {
            req.value = Some(bad.into());
            let reply = h.app.ctl_response(req.clone()).0;
            assert!(!reply.ok, "{bad} should be refused");
            assert_eq!(h.app.settings.scroll_lines, 7, "{bad} changed the value");
        }
    }

    #[test]
    fn scroll_distance_follows_the_setting() {
        let mut h = harness(Mode::Tree);
        h.app.rows_len = 1000;
        h.app.settings.scroll_lines = 1;
        h.app.scroll_rows(h.app.settings.scroll_lines as isize, 10);
        assert_eq!(h.app.view.scroll, 1);
        h.app.settings.scroll_lines = 10;
        h.app.scroll_rows(h.app.settings.scroll_lines as isize, 10);
        assert_eq!(h.app.view.scroll, 11);
    }

    // ---- ADR 0025: a click completes on release ----

    /// Arms a press on `idx` and returns the hit tuple a release would carry.
    fn press(app: &mut App, rows: &[Row], idx: usize, on_chevron: bool) {
        app.armed_press = Some((rows[idx].path.clone(), on_chevron));
    }

    #[test]
    fn press_alone_selects_nothing() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a", NodeKind::File), ("b", NodeKind::File)],
        );
        let rows = h.app.rows();
        let before = h.app.view.selection.clone();
        press(&mut h.app, &rows, 2, false);
        assert_eq!(h.app.view.selection, before, "a press must not select");
    }

    #[test]
    fn release_on_the_pressed_row_selects() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a", NodeKind::File), ("b", NodeKind::File)],
        );
        let rows = h.app.rows();
        press(&mut h.app, &rows, 2, false);
        h.app
            .resolve_release(&rows, Some((2, false)), Instant::now());
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(rows[2].path.as_path())
        );
        assert!(h.app.armed_press.is_none(), "the arm is spent");
    }

    #[test]
    fn release_on_a_different_row_does_nothing() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a", NodeKind::File), ("b", NodeKind::File)],
        );
        let rows = h.app.rows();
        let before = h.app.view.selection.clone();
        press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, Some((2, false)), Instant::now());
        assert_eq!(
            h.app.view.selection, before,
            "sliding off the pressed row revokes the click"
        );
        assert!(h.app.armed_press.is_none());
    }

    #[test]
    fn release_outside_the_tree_does_nothing() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a", NodeKind::File)]);
        let rows = h.app.rows();
        let before = h.app.view.selection.clone();
        press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(&rows, None, Instant::now());
        assert_eq!(h.app.view.selection, before);
    }

    #[test]
    fn a_release_with_no_press_does_nothing() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a", NodeKind::File)]);
        let rows = h.app.rows();
        let before = h.app.view.selection.clone();
        h.app
            .resolve_release(&rows, Some((1, false)), Instant::now());
        assert_eq!(h.app.view.selection, before);
    }

    #[test]
    fn a_press_on_the_chevron_released_on_the_name_does_not_toggle() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("sub", NodeKind::Dir)]);
        feed(&mut h.app, "/r/sub", &[("f", NodeKind::File)]);
        let rows = h.app.rows();
        let expanded = h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded;
        press(&mut h.app, &rows, 1, true);
        h.app
            .resolve_release(&rows, Some((1, false)), Instant::now());
        assert_eq!(
            h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded,
            expanded,
            "the zones differ, so the click is abandoned"
        );
    }

    #[test]
    fn two_complete_clicks_activate_but_press_release_press_does_not() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("a", NodeKind::File)]);
        let rows = h.app.rows();
        let t0 = Instant::now();
        press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(&rows, Some((1, false)), t0);
        assert!(h.app.picked.is_none(), "one click only selects");
        // A second press without its release must not activate.
        press(&mut h.app, &rows, 1, false);
        assert!(h.app.picked.is_none());
        h.app
            .resolve_release(&rows, Some((1, false)), t0 + Duration::from_millis(100));
        assert_eq!(
            h.app.picked.as_deref(),
            Some(rows[1].path.as_path()),
            "two complete clicks activate"
        );
    }

    #[test]
    fn the_double_click_window_is_measured_release_to_release() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("a", NodeKind::File)]);
        let rows = h.app.rows();
        let t0 = Instant::now();
        press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(&rows, Some((1, false)), t0);
        press(&mut h.app, &rows, 1, false);
        // Beyond the window: a slow second click is two singles, not a double.
        h.app.resolve_release(
            &rows,
            Some((1, false)),
            t0 + input::DOUBLE_CLICK_WINDOW + Duration::from_millis(1),
        );
        assert!(h.app.picked.is_none());
    }

    #[test]
    fn an_abandoned_click_clears_a_pending_double() {
        let mut h = harness(Mode::Pick);
        feed(
            &mut h.app,
            "/r",
            &[("a", NodeKind::File), ("b", NodeKind::File)],
        );
        let rows = h.app.rows();
        let t0 = Instant::now();
        press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(&rows, Some((1, false)), t0);
        // A press released elsewhere is an intervening click: it resets.
        press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, Some((2, false)), t0 + Duration::from_millis(50));
        press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, Some((1, false)), t0 + Duration::from_millis(100));
        assert!(
            h.app.picked.is_none(),
            "the abandoned click reset the double"
        );
    }
}

#[cfg(test)]
mod ctl_tests {
    use birch_core::NodeKind;
    use birch_core::protocol::{PathForm, Request, SettingKey, Verb};

    use super::tests::{drain_expands, feed, harness};
    use super::*;

    fn request(verb: Verb) -> Request {
        Request::new(verb)
    }

    #[test]
    fn reveal_validates_the_root_boundary_lexically() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);

        let mut req = request(Verb::Reveal);
        req.path = Some("src/../src/main.rs".into());
        let (resp, _) = h.app.ctl_response(req);
        assert!(resp.ok, "dot-dot inside the root is fine");

        let mut req = request(Verb::Reveal);
        req.path = Some("/r/../etc/passwd".into());
        let (resp, _) = h.app.ctl_response(req);
        assert!(!resp.ok, "dot-dot escaping the root is rejected");

        let mut req = request(Verb::Reveal);
        req.path = Some("relative.txt".into());
        let (resp, _) = h.app.ctl_response(req);
        assert!(resp.ok, "relative paths resolve against the root");
    }

    #[test]
    fn resolve_within_root_handles_symlinked_prefix() {
        // A real dir plus a symlink pointing at it (mirrors macOS /tmp →
        // /private/tmp). Reveal via the symlinked prefix must resolve.
        let base = std::env::temp_dir().join(format!("birch-reveal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("real/sub")).unwrap();
        std::fs::write(base.join("real/sub/main.rs"), b"x").unwrap();
        let base = base.canonicalize().unwrap();
        let real = base.join("real");
        let link = base.join("alias");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        // Already under the root: returned as given, no symlink resolution.
        assert_eq!(
            resolve_within_root(&real, Path::new("sub/main.rs")),
            Some(real.join("sub/main.rs"))
        );
        // Symlinked prefix: resolved via the fallback to the canonical path.
        assert_eq!(
            resolve_within_root(&real, &link.join("sub/main.rs")),
            Some(real.join("sub/main.rs"))
        );
        // A not-yet-existing leaf under the symlinked prefix still resolves.
        assert_eq!(
            resolve_within_root(&real, &link.join("sub/new.rs")),
            Some(real.join("sub/new.rs"))
        );
        // A genuinely outside path is rejected.
        assert_eq!(resolve_within_root(&real, Path::new("/etc/passwd")), None);

        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn get_path_forms_and_root_dot() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        h.app.view.focus("/r/src".into());

        let mut req = request(Verb::GetPath);
        req.form = Some(PathForm::Name);
        assert_eq!(h.app.ctl_response(req).0.data.as_deref(), Some("src"));
        let mut req = request(Verb::GetPath);
        req.form = Some(PathForm::Abs);
        assert_eq!(h.app.ctl_response(req).0.data.as_deref(), Some("/r/src"));
        let req = request(Verb::GetPath); // default form = rel
        assert_eq!(h.app.ctl_response(req).0.data.as_deref(), Some("src"));

        // The root itself prints "." rather than an empty line.
        h.app.view.focus("/r".into());
        let req = request(Verb::GetPath);
        assert_eq!(h.app.ctl_response(req).0.data.as_deref(), Some("."));

        assert_eq!(
            h.app.ctl_response(request(Verb::GetRoot)).0.data.as_deref(),
            Some("/r")
        );
    }

    #[test]
    fn set_toggles_settings_and_git_off_clears_state() {
        let mut h = harness(Mode::Tree);
        assert!(h.app.settings.show_hidden);
        let mut req = request(Verb::Set);
        req.setting = Some(SettingKey::Hidden);
        req.value = Some("toggle".into());
        assert!(h.app.ctl_response(req).0.ok);
        assert!(!h.app.settings.show_hidden);

        h.app.git_state = Some(Arc::new(birch_core::git::parse_porcelain_v2(
            b"? x\0",
            Path::new("/r"),
        )));
        let mut req = request(Verb::Set);
        req.setting = Some(SettingKey::Git);
        req.value = Some("off".into());
        assert!(h.app.ctl_response(req).0.ok);
        assert!(h.app.git_state.is_none(), "stale decorations cleared");

        let mut req = request(Verb::Set);
        req.setting = Some(SettingKey::Hidden);
        req.value = Some("maybe".into());
        assert!(!h.app.ctl_response(req).0.ok, "bad value rejected");
    }

    #[test]
    fn set_theme_selects_by_id_and_rejects_unknown() {
        let mut h = harness(Mode::Tree);
        assert_eq!(h.app.settings.theme, ThemeId::Birch);

        let mut req = request(Verb::Set);
        req.setting = Some(SettingKey::Theme);
        req.value = Some("plain".into());
        assert!(h.app.ctl_response(req).0.ok);
        assert_eq!(h.app.settings.theme, ThemeId::Plain);

        // An unknown theme id errors and leaves the current theme unchanged.
        let mut req = request(Verb::Set);
        req.setting = Some(SettingKey::Theme);
        req.value = Some("neon".into());
        assert!(!h.app.ctl_response(req).0.ok, "unknown theme rejected");
        assert_eq!(h.app.settings.theme, ThemeId::Plain);
    }

    #[test]
    fn open_and_quit_effects_are_deferred() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("a.txt", NodeKind::File)],
        );

        let (resp, effect) = h.app.ctl_response(request(Verb::Open));
        assert!(!resp.ok, "no selection yet");
        assert!(matches!(effect, CtlEffect::None));

        h.app.view.focus("/r/a.txt".into());
        let (resp, effect) = h.app.ctl_response(request(Verb::Open));
        assert!(resp.ok);
        assert!(matches!(effect, CtlEffect::Open(p) if p == Path::new("/r/a.txt")));

        // Open on a dir expands instead.
        h.app.view.focus("/r/src".into());
        drain_expands(&h.source_rx);
        let (resp, effect) = h.app.ctl_response(request(Verb::Open));
        assert!(resp.ok);
        assert!(matches!(effect, CtlEffect::None));
        assert!(h.app.tree.node_at(Path::new("/r/src")).unwrap().expanded);

        let (resp, effect) = h.app.ctl_response(request(Verb::Quit));
        assert!(resp.ok);
        assert!(matches!(effect, CtlEffect::Quit));
    }

    #[test]
    fn set_root_rebinds_everything() {
        let tmp = std::env::temp_dir().join(format!("birch-setroot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("inner")).unwrap();
        let tmp = tmp.canonicalize().unwrap();

        let mut h = harness(Mode::Pick); // no persistence I/O
        feed(&mut h.app, "/r", &[("old.txt", NodeKind::File)]);
        h.app.view.focus("/r/old.txt".into());
        h.app.search = None;
        drain_expands(&h.source_rx);

        let mut req = request(Verb::SetRoot);
        req.path = Some(tmp.clone());
        let (resp, _) = h.app.ctl_response(req);
        assert!(resp.ok, "{resp:?}");
        assert_eq!(h.app.root, tmp);
        assert!(h.app.tree.node_at(Path::new("/r/old.txt")).is_none());
        assert!(h.app.view.selection.is_none());
        assert_eq!(drain_expands(&h.source_rx), std::slice::from_ref(&tmp));

        // A file target is rejected.
        std::fs::write(tmp.join("f"), b"x").unwrap();
        let mut req = request(Verb::SetRoot);
        req.path = Some(tmp.join("f"));
        assert!(!h.app.ctl_response(req).0.ok);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
