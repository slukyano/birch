//! The app loop: one `recv()` over the unified event channel; deltas mutate
//! the tree, input becomes actions, every iteration reconciles watches,
//! requests peek-loads, persists expansion changes, and redraws.

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
use birch_core::search::{IndexCmd, IndexEvent, Match, SearchIndex, search};
use birch_core::watcher::{WatchCmd, WatchEvent};
use birch_core::{
    NodeKind, OpenCmd, OpenMode, Settings, SourceCmd, SourceEvent, Tree, TreeDelta, persist,
};
use birch_tui::flat_view::{self, Decor, FlatView, NavEffect, Row};
use birch_tui::input::{self, InputAction};
use birch_tui::render;
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
    pub input_paused: Arc<AtomicBool>,
}

/// Active fuzzy search (ADR 0009). In tree mode the pane jumps over matches;
/// in picker mode the matches replace the rows.
struct SearchState {
    query: String,
    matches: Vec<Match>,
    /// Path → matched char indices into the simple name (empty for path-mode
    /// hits, meaning whole-name highlight).
    matched_set: HashMap<PathBuf, Vec<u32>>,
    current: usize,
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
}

/// Runs the app; in picker mode the returned path is the confirmed pick.
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
        input_paused,
    } = wiring;

    let mut app = App {
        tree: Tree::new(root.clone(), settings.files_first),
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
                self.rematch(true);
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

        let rows = self.rows();
        let viewport = render::tree_viewport_height(area(terminal));
        // With a live search in tree mode, ↑/↓ cycle the matches.
        if self.mode == Mode::Tree
            && let Some(state) = &self.search
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
            InputAction::Right => {
                if self.filter_list_active() {
                    NavEffect::None
                } else {
                    self.view.on_right(&mut self.tree, &rows)
                }
            }
            InputAction::Left => {
                if !self.filter_list_active() {
                    self.view.on_left(&mut self.tree, &rows);
                }
                NavEffect::None
            }
            InputAction::Enter => self.activate(&rows, None),
            InputAction::ScrollUp => {
                self.view.scroll_by(&rows, -input::SCROLL_LINES, viewport);
                NavEffect::None
            }
            InputAction::ScrollDown => {
                self.view.scroll_by(&rows, input::SCROLL_LINES, viewport);
                NavEffect::None
            }
            InputAction::Click { column, row } => {
                match render::hit_test(&rows, &self.view, area(terminal), column, row) {
                    Some((idx, on_chevron)) => {
                        self.resolve_click(&rows, idx, on_chevron, Instant::now())
                    }
                    None => {
                        // Any intervening click resets a pending double.
                        self.click_timer.disarm();
                        NavEffect::None
                    }
                }
            }
            InputAction::Redraw
            | InputAction::Quit
            | InputAction::Char(_)
            | InputAction::Backspace
            | InputAction::Esc => NavEffect::None,
        };
        // These actions may toggle expansion; the saver diffs the actual set
        // and skips no-op writes, so over-marking is cheap.
        if matches!(
            action,
            InputAction::Right | InputAction::Left | InputAction::Enter | InputAction::Click { .. }
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
                Some(path) => {
                    let abs = if path.is_absolute() {
                        path
                    } else {
                        self.root.join(path)
                    };
                    // Lexical normalization, no canonicalize: the tree speaks
                    // the paths as listed (a symlinked entry inside the tree
                    // must not resolve outside it), while `..` segments must
                    // not escape the root check.
                    let abs = lexical_normalize(&abs);
                    if abs.starts_with(&self.root) {
                        self.reveal(abs);
                        Response::ok(None)
                    } else {
                        Response::err("path is outside the root")
                    }
                }
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
            SettingKey::FilesFirst => {
                self.settings.files_first = value.apply(self.settings.files_first);
                self.tree.set_files_first(self.settings.files_first);
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
        self.tree = Tree::new(new_root.clone(), self.settings.files_first);
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
    /// double-click activates. The flat filter list has no tree semantics,
    /// so its chevron zone counts as the name there — otherwise a single
    /// click on a dir match could confirm a pick.
    fn resolve_click(
        &mut self,
        rows: &[Row],
        idx: usize,
        on_chevron: bool,
        now: Instant,
    ) -> NavEffect {
        if on_chevron && !self.filter_list_active() {
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
    /// never confirm by accident. On the filter list a double-click picks
    /// (it is a selection list).
    fn activate(&mut self, rows: &[Row], click: Option<(usize, bool)>) -> NavEffect {
        let idx = match click {
            Some((idx, _)) => Some(idx),
            None => self.view.sync(rows),
        };
        if self.mode.is_pick()
            && let Some(idx) = idx
            && let Some(row) = rows.get(idx)
            && !row.missing
        {
            let browsing_click = click.is_some() && row.kind.is_dir() && !self.filter_list_active();
            if !browsing_click {
                if click.is_some() {
                    // A picking click also moves the selection there.
                    self.view.focus(row.path.clone());
                }
                self.picked = Some(row.path.clone());
                return NavEffect::None;
            }
        }
        if self.filter_list_active() {
            // No tree semantics on the flat list beyond confirming.
            return NavEffect::None;
        }
        match click {
            Some((idx, on_chevron)) => self.view.on_click(&mut self.tree, rows, idx, on_chevron),
            None => self.view.on_enter(&mut self.tree, rows),
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
                current: 0,
                saved_selection: self.view.selection.clone(),
                saved_scroll: self.view.scroll,
            });
        }
        if let Some(state) = &mut self.search {
            state.query.push(c);
        }
        self.rematch(false);
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
        self.rematch(false);
    }

    /// Esc backs out one layer (ADR 0012): an active search clears (tree
    /// mode restores the pre-search view); with nothing to dismiss, Esc
    /// quits — the picker without a pick, the tree like Ctrl-C.
    fn on_esc(&mut self) -> bool {
        match self.search.take() {
            Some(state) => {
                if self.mode == Mode::Tree {
                    self.view.selection = state.saved_selection;
                    self.view.scroll = state.saved_scroll;
                }
                self.pending_reveal = None;
                false
            }
            None => {
                self.pending_reveal = None;
                true
            }
        }
    }

    /// Recomputes matches for the current query. `keep_position` preserves
    /// the current match pointer (used when a fresh index arrives).
    fn rematch(&mut self, keep_position: bool) {
        let Some(state) = &mut self.search else {
            return;
        };
        let Some(index) = &self.index else {
            state.matches = Vec::new();
            state.matched_set = HashMap::new();
            return;
        };
        state.matches = search(index, &state.query);
        state.matched_set = state
            .matches
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
        if !keep_position {
            state.current = 0;
        }
        state.current = state.current.min(state.matches.len().saturating_sub(1));
        if self.mode == Mode::Tree {
            // Reveal the current match — after an index rebuild that is the
            // match the user cycled to, not necessarily the best one.
            let target = self
                .search
                .as_ref()
                .and_then(|s| s.matches.get(s.current))
                .map(|m| m.entry.abs.clone());
            if let Some(target) = target {
                self.reveal(target);
            }
        } else if let Some(state) = &self.search
            && let Some(first) = state.matches.first()
        {
            // Filter list: selection snaps to the top match.
            self.view.focus(first.entry.abs.clone());
            self.view.scroll = 0;
        }
    }

    fn cycle_match(&mut self, forward: bool) {
        let Some(state) = &mut self.search else {
            return;
        };
        let n = state.matches.len();
        if n == 0 {
            return;
        }
        state.current = if forward {
            (state.current + 1) % n
        } else {
            (state.current + n - 1) % n
        };
        let target = state.matches[state.current].entry.abs.clone();
        self.reveal(target);
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

    fn filter_list_active(&self) -> bool {
        self.mode.is_pick() && self.search.as_ref().is_some_and(|s| !s.query.is_empty())
    }

    fn rows(&self) -> Vec<Row> {
        let git = if self.settings.git {
            self.git_state.as_deref()
        } else {
            None
        };
        if self.filter_list_active() {
            let state = self.search.as_ref().expect("filter list implies search");
            return flat_view::match_rows(&state.matches, git);
        }
        let matched = if self.mode == Mode::Tree {
            self.search.as_ref().map(|s| &s.matched_set)
        } else {
            None
        };
        flat_view::visible_rows(
            &self.tree,
            &self.settings,
            Decor {
                git,
                matched,
                home: self.home.as_deref(),
                split: Some(&self.view.split),
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
        let (view, settings) = (&self.view, &self.settings);
        terminal.draw(|frame| render::draw(frame, &rows, view, settings, &bottom))?;
        Ok(())
    }

    fn bottom_line(&self) -> String {
        let base = if let Some(state) = &self.search {
            let n = state.matches.len();
            match self.mode {
                Mode::Tree if n == 0 => format!("search: {} (no matches)", state.query),
                Mode::Tree => format!("search: {} ({}/{})", state.query, state.current + 1, n),
                Mode::Pick => format!("> {} ({n} matches)", state.query),
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
        if !self.settings.compact || self.filter_list_active() {
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
            tree: Tree::new(root.clone(), false),
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
    fn cycle_wraps_and_index_refresh_keeps_current() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a.txt", NodeKind::File), ("ab.txt", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("a.txt", false), ("ab.txt", false)]));
        h.app.search_push('a');
        let n = h.app.search.as_ref().unwrap().matches.len();
        assert_eq!(n, 2);

        h.app.cycle_match(true);
        assert_eq!(h.app.search.as_ref().unwrap().current, 1);
        h.app.cycle_match(true);
        assert_eq!(h.app.search.as_ref().unwrap().current, 0);
        h.app.cycle_match(false);
        assert_eq!(h.app.search.as_ref().unwrap().current, 1);

        // A fresh index (watcher churn) keeps the cycled position and reveals
        // the match at that position, not the best one.
        let current_target = h.app.search.as_ref().unwrap().matches[1].entry.abs.clone();
        h.app.index = Some(index_of(&[("a.txt", false), ("ab.txt", false)]));
        h.app.rematch(true);
        assert_eq!(h.app.search.as_ref().unwrap().current, 1);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(current_target.as_path())
        );
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
    fn filter_list_single_click_never_picks() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        h.app.index = Some(index_of(&[("src", true)]));
        h.app.search_push('s');
        let rows = h.app.rows();
        assert_eq!(rows[0].name, "src");
        let t0 = Instant::now();
        // The flat list draws no real chevron, so its chevron zone is the
        // name: a single click on a dir match selects — it must never
        // confirm the pick (sprint-010 review finding).
        assert!(matches!(
            h.app.resolve_click(&rows, 0, true, t0),
            NavEffect::None
        ));
        assert!(h.app.picked.is_none());
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/src")));
        // The completed double-click picks.
        h.app
            .resolve_click(&rows, 0, false, t0 + Duration::from_millis(100));
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/src")));
    }

    #[test]
    fn picker_filter_list_shows_matches_flat() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        h.app.index = Some(index_of(&[("src/main.rs", false), ("src", true)]));
        h.app.search_push('m');
        let rows = h.app.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "src/main.rs");
        assert_eq!(rows[0].depth, 0);

        // Confirming on the flat list picks the absolute path.
        h.app.view.focus("/r/src/main.rs".into());
        let rows = h.app.rows();
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_deref(), Some(Path::new("/r/src/main.rs")));
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
