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

use birch_core::action::{Action, ActionContext, ActionTarget, CopyForm};
use birch_core::clipboard;
use birch_core::git::{self, GitCmd, GitEvent, GitState};
use birch_core::ops;
use birch_core::protocol::{PathForm, Request, Response, SettingKey, SettingValue, Verb};
use birch_core::search::{self, IndexCmd, IndexEvent, Match, SearchIndex, search};
use birch_core::watcher::{WatchCmd, WatchEvent};
use birch_core::{
    Filter, NodeKind, OpenCmd, OpenMode, Settings, SourceCmd, SourceEvent, ThemeId, Tree,
    TreeDelta, persist, settings, undo,
};
use birch_tui::edit::{EditKind, Editor};
use birch_tui::flat_view::{self, Decor, FlatView, NavEffect, Row, SelectionSnapshot};
use birch_tui::input::{self, InputAction};
use birch_tui::menu::Menu;
use birch_tui::render;
use birch_tui::theme::Theme;
use ratatui::layout::Rect;

use crate::ctl::{CtlRequest, SocketHandle};
use crate::{AppEvent, OpenEvent, term};

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
    /// `--pick --multi` (task 074). Ignored outside picker mode, where
    /// multi-select is always available.
    pub multi: bool,
    pub events: Receiver<AppEvent>,
    pub source_cmds: Sender<SourceCmd>,
    pub watch_cmds: Sender<WatchCmd>,
    pub index_cmds: Sender<IndexCmd>,
    pub git_cmds: Sender<GitCmd>,
    /// Where a detached open's reaper thread reports a failure (task 085).
    pub open_events: Sender<OpenEvent>,
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
    saved_selection: SelectionSnapshot,
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
    open_events: Sender<OpenEvent>,
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
    /// Confirmed paths, in tree order. Empty until `Enter` picks; more than
    /// one only under `--multi`.
    picked: Vec<PathBuf>,
    /// `--pick --multi`: multi-select is enabled in the picker and every
    /// selected path is printed. Off, the picker behaves exactly as it always
    /// has, and a selection the mode cannot output is not offered.
    multi: bool,
    input_paused: Arc<AtomicBool>,
    click_timer: input::ClickTimer,
    /// The open context menu, if any. Holds ordering and a highlight; what its
    /// entries *do* lives in the action layer (ADR 0029).
    menu: Option<Menu>,
    /// A right-press waiting for its release, holding the row it resolved to —
    /// `Some(None)` for a press below the last row, which targets the root.
    /// Keyed on the row, not the cell, so a pointer that drifts a column
    /// between press and release still opens the menu (ADR 0025).
    armed_right: Option<Option<PathBuf>>,
    /// The open inline editor: birch's only mode (ADR 0030). While one is open
    /// printables reach it rather than the search, because they go to whatever
    /// text field is focused.
    edit: Option<Editor>,
    /// A selection held for a paste: the paths, and whether pasting moves them
    /// or duplicates them. Session-only and single-slot, like every clipboard.
    pending: Option<Pending>,
    /// `pending` described, kept beside it so building an `ActionContext`
    /// borrows a string instead of formatting one.
    pending_label: Option<String>,
    /// What birch has done that it can take back (task 093). Session-only: it
    /// goes with the process, by design rather than by omission.
    history: undo::History,
    /// The next step's description, kept beside the history so building an
    /// `ActionContext` — which happens on every menu open and every hotkey —
    /// borrows a string instead of formatting one.
    undo_label: Option<String>,
    /// A drag in flight: the paths being moved, and the directory currently
    /// under the pointer. A drag begins when the pointer leaves the row the
    /// press started on — ADR 0025 already tracks which row that was, so there
    /// is no threshold to tune (ADR 0032).
    drag: Option<Drag>,
    /// A `Shift` extension waiting for its match reveal to converge. The range
    /// is painted once, when the cursor lands, rather than every frame.
    pending_extend: bool,
    /// The armed press: which row a left button-down landed on, and whether it
    /// landed in the chevron zone. A click completes only when the release
    /// matches both (ADR 0025). Keyed on the real path, so a snapshot arriving
    /// between press and release cannot redirect the click.
    armed_press: Option<(PathBuf, bool)>,
    /// When the match list was last recomputed from a *growing* index.
    last_rematch: Option<Instant>,
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

/// How often the match list is recomputed while the index is still growing
/// (ADR 0026). Fast enough that results visibly improve, rare enough that the
/// walk is not spent re-scanning what it has already found.
const INDEX_REMATCH_INTERVAL: Duration = Duration::from_millis(250);

/// Whether the batch may continue, given what the last handled event did.
/// One rule, asked both before a batch opens and after each event inside it,
/// so the two can never disagree (ADR 0024, decision 4).
fn batch_step(quit: bool, yielded_terminal: bool) -> BatchStep {
    if quit {
        BatchStep::Quit
    } else if yielded_terminal {
        // A child owned the tty; queued events must not run behind it, and the
        // screen it left needs the redraw now.
        BatchStep::Stop
    } else {
        BatchStep::Continue
    }
}

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
/// batch, so an idle birch still draws one frame per event. `None` drains the
/// queue outright — the post-handover drain must leave nothing behind.
fn drain_batch<F>(
    events: &Receiver<AppEvent>,
    deadline: Option<Instant>,
    mut handle: F,
) -> BatchStop
where
    F: FnMut(AppEvent) -> BatchStep,
{
    loop {
        if deadline.is_some_and(|at| Instant::now() >= at) {
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

/// A move in flight.
struct Drag {
    /// What is being moved, resolved when the drag began.
    paths: Vec<PathBuf>,
    /// The directory under the pointer, and whether it will accept the drop.
    /// `None` means the pointer is somewhere that is not a target at all.
    target: Option<DropTarget>,
}

struct DropTarget {
    directory: PathBuf,
    refusal: Option<String>,
}

/// A selection held for a paste. `duplicate` is what tells a cut from a copy;
/// nothing else about them differs.
struct Pending {
    paths: Vec<PathBuf>,
    duplicate: bool,
}

/// What performing an action leaves for the caller to do. `perform` stays free
/// of the terminal — opening a file hands the tty over, and an OSC 52 copy is a
/// sequence only the render layer can write.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Performed {
    Nothing,
    /// A file the caller should open with the configured command.
    Open(PathBuf),
    /// A path the caller should open with a specific command — the platform
    /// opener, for `Open system`.
    OpenWith(PathBuf, OpenCmd),
    /// An escape sequence the caller should write to the terminal.
    Write(String),
}

/// Names a set of paths for a message: one by name, several by count.
/// The path a delta announced. How the app learns what an operation actually
/// produced, without the op layer growing a second return value for undo's
/// benefit — and it is the *announced* path, so a create that made
/// `a/b/c.txt` from nothing reports `a`, which is what taking it back means.
fn added_path(delta: &TreeDelta) -> Option<PathBuf> {
    match delta {
        TreeDelta::Added { parent, entries } => {
            entries.first().map(|entry| parent.join(&entry.name))
        }
        _ => None,
    }
}

fn describe(paths: &[PathBuf]) -> String {
    match paths {
        [one] => one.file_name().map_or_else(
            || one.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        many => format!("{} items", many.len()),
    }
}

/// What was copied, for the status line. The count matters and the form is what
/// distinguishes three otherwise identical actions.
fn copied_message(count: usize, form: CopyForm) -> String {
    let what = match form {
        CopyForm::Name => "name",
        CopyForm::Relative => "relative path",
        CopyForm::Absolute => "absolute path",
    };
    if count == 1 {
        format!("Copied {what}")
    } else {
        format!("Copied {count} {what}s")
    }
}

/// What an input asks of the open menu. Keeps the decision separable from the
/// I/O it implies, so every branch is testable without a terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
enum MenuStep {
    /// The menu stays open; any highlight move has already been applied.
    Keep,
    Close,
    /// Close, and send the character to the search.
    Search(char),
    /// Close, and perform the action.
    Invoke(Action),
    /// Close, and arm a right-press so the release opens a menu elsewhere.
    Reopen(Option<Option<PathBuf>>),
}

/// The open menu. Only called from paths that have just established there is
/// one, so the panic is unreachable rather than a policy.
fn menu_mut(menu: &mut Option<Menu>) -> &mut Menu {
    menu.as_mut().expect("the menu is open")
}

fn menu_ref(menu: &Option<Menu>) -> &Menu {
    menu.as_ref().expect("the menu is open")
}

pub fn run(terminal: &mut term::Term, wiring: AppWiring) -> io::Result<Vec<PathBuf>> {
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
        open_events,
        repo_root,
        socket,
        filter,
        input_paused,
        multi,
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
        open_events,
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
        picked: Vec::new(),
        multi,
        input_paused,
        click_timer: input::ClickTimer::default(),
        menu: None,
        armed_right: None,
        edit: None,
        history: undo::History::default(),
        undo_label: None,
        pending: None,
        pending_label: None,
        drag: None,
        pending_extend: false,
        armed_press: None,
        last_rematch: None,
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
        // A hand-off in the event just handled ends the batch before it starts
        // — nothing may run behind a child that owned the tty (decision 4).
        let stopped = if batch_step(false, app.yielded_terminal) == BatchStep::Continue {
            drain_batch(&events, Some(Instant::now() + BATCH_BUDGET), |event| {
                let quit = app.handle(terminal, &events, event);
                batch_step(quit, app.yielded_terminal)
            })
        } else {
            BatchStop::Handoff
        };
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
                // A walk started for a previous root keeps publishing after a
                // re-root; adopting it would describe the wrong corpus, and
                // claim to be complete about it.
                if index.root() != self.root {
                    return false;
                }
                let complete = index.is_complete();
                self.index = Some(index);
                // A growing index publishes ~100 times on a large root, and
                // rematching on every one doubles the walk while a query is
                // live. The match list has to keep improving, not track every
                // chunk — so intermediate ones rematch on a cadence, and the
                // final one always does.
                let due = complete
                    || self
                        .last_rematch
                        .is_none_or(|at| at.elapsed() >= INDEX_REMATCH_INTERVAL);
                if due {
                    self.last_rematch = Some(Instant::now());
                    // The corpus grew; the query did not change. New matches
                    // become steppable, but the cursor stays where it is.
                    self.rematch_keeping_selection();
                }
            }
            AppEvent::Ctl(ctl) => return self.handle_ctl(terminal, events, ctl),
            // A detached open that failed. Last writer wins, like every
            // other status message, and the keypress that triggered the open
            // has already cleared the line — so this lands after its cause.
            AppEvent::Open(failure) => self.status = open_failure_message(&failure),
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

        // Layers see input before the tree, outermost first: an edit, then the
        // menu. `Esc` therefore backs out of them in that order (ADR 0012,
        // extended by ADR 0030).
        if self.edit.is_some() {
            self.handle_edit_input(action);
            return false;
        }
        if self.menu.is_some() {
            return self.handle_menu_input(terminal, events, action);
        }

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

        // Without a plural selection to build, `Shift` with a movement key is
        // just that movement: the picker without `--multi` behaves exactly as
        // it always has, rather than building a set it cannot output.
        let action = match action {
            InputAction::Extend(movement) if !self.selection_is_plural() => match movement {
                input::Movement::Up => InputAction::Up,
                input::Movement::Down => InputAction::Down,
                input::Movement::PageUp => InputAction::PageUp,
                input::Movement::PageDown => InputAction::PageDown,
                input::Movement::Home => InputAction::Home,
                input::Movement::End => InputAction::End,
            },
            other => other,
        };

        let rows = self.rows();
        // One page is the viewport less a row of overlap: the row at the bottom
        // edge becomes the row at the top, which is the only reason a page is
        // not simply the viewport. The floor keeps a one-row pane usable.
        let page = render::tree_viewport_height(area(terminal))
            .saturating_sub(1)
            .max(1) as isize;
        // With a live search every movement key moves by *matches*, because
        // nothing else is selectable (ADR 0023) — in both modes, since the
        // picker is the same tree. Single steps wrap; page and edge steps clamp
        // (task 081).
        let match_count = self.search.as_ref().map_or(0, |state| state.matches.len()) as isize;
        if match_count > 0 {
            // `Shift` extends over matches, by the same unit rule: under a
            // narrowing the thing a movement key moves by is a match.
            let extend = matches!(action, InputAction::Extend(_));
            let movement = match action {
                InputAction::Extend(movement) => Some(movement),
                InputAction::Up => Some(input::Movement::Up),
                InputAction::Down => Some(input::Movement::Down),
                InputAction::PageUp => Some(input::Movement::PageUp),
                InputAction::PageDown => Some(input::Movement::PageDown),
                InputAction::Home => Some(input::Movement::Home),
                InputAction::End => Some(input::Movement::End),
                _ => None,
            };
            if let Some(movement) = movement {
                let (delta, wrap) = match movement {
                    input::Movement::Up => (-1, true),
                    input::Movement::Down => (1, true),
                    input::Movement::PageUp => (-page, false),
                    input::Movement::PageDown => (page, false),
                    input::Movement::Home => (-match_count, false),
                    input::Movement::End => (match_count, false),
                };
                // Stepping reaches its target over several frames, so the
                // extension is armed here and painted once the cursor lands.
                if extend {
                    self.view.begin_extending(&rows);
                    self.pending_extend = true;
                } else {
                    self.view.end_extending();
                    self.pending_extend = false;
                }
                self.step_match(delta, wrap);
                return false;
            }
        }
        // Every unmodified movement collapses the selection to the cursor. Done
        // here rather than inside the movement so it holds when the move goes
        // nowhere — already at an edge, or `→` with nothing left to reveal.
        if matches!(
            action,
            InputAction::Up
                | InputAction::Down
                | InputAction::Right
                | InputAction::Left
                | InputAction::PageUp
                | InputAction::PageDown
                | InputAction::Home
                | InputAction::End
        ) {
            self.view.collapse_selection();
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
            // Paging moves the selection and lets the viewport follow; the
            // wheel stays the only motion that moves the viewport alone
            // (task 081). `move_by` counts live rows and clamps at the ends.
            InputAction::PageUp => {
                self.view.move_by(&rows, -page);
                NavEffect::None
            }
            InputAction::PageDown => {
                self.view.move_by(&rows, page);
                NavEffect::None
            }
            InputAction::Home => {
                self.view.move_to_edge(&rows, true);
                NavEffect::None
            }
            InputAction::End => {
                self.view.move_to_edge(&rows, false);
                NavEffect::None
            }
            // `Shift` with a movement key extends the range from the anchor
            // (task 074); the movement itself is the same one the bare key
            // makes, so the two can never disagree about where the cursor goes.
            InputAction::Extend(movement) => {
                match movement {
                    input::Movement::Up => self.view.move_by_extending(&rows, -1),
                    input::Movement::Down => self.view.move_by_extending(&rows, 1),
                    input::Movement::PageUp => self.view.move_by_extending(&rows, -page),
                    input::Movement::PageDown => self.view.move_by_extending(&rows, page),
                    input::Movement::Home => self.view.move_to_edge_extending(&rows, true),
                    input::Movement::End => self.view.move_to_edge_extending(&rows, false),
                }
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
                let hit = render::hit_test(
                    &rows,
                    &self.view,
                    render::Chrome {
                        settings: &self.settings,
                        theme: &Theme::for_id(self.settings.theme),
                        branch_axis: self.branch_axis(),
                    },
                    area(terminal),
                    column,
                    row,
                );
                self.arm_press(&rows, hit);
                NavEffect::None
            }
            InputAction::Drag { column, row } => {
                self.update_drag(area(terminal), &rows, column, row);
                NavEffect::None
            }
            InputAction::Release {
                column,
                row,
                modifier,
            } => {
                let hit = render::hit_test(
                    &rows,
                    &self.view,
                    render::Chrome {
                        settings: &self.settings,
                        theme: &Theme::for_id(self.settings.theme),
                        branch_axis: self.branch_axis(),
                    },
                    area(terminal),
                    column,
                    row,
                );
                // A drag in flight ends here rather than completing a click:
                // the press was consumed by the gesture.
                if self.drag.is_some() {
                    self.finish_drag();
                    NavEffect::None
                } else {
                    self.resolve_release(&rows, hit, modifier, Instant::now())
                }
            }
            // The press only arms: the menu opens on the release, on the row
            // the press started on — the left button's rule with no exception
            // carved for the right one (ADR 0025, extended by ADR 0029).
            InputAction::RightPress { column, row } => {
                self.armed_right = self.right_target(area(terminal), &rows, column, row);
                NavEffect::None
            }
            InputAction::RightRelease { column, row } => {
                let armed = self.armed_right.take();
                let released = self.right_target(area(terminal), &rows, column, row);
                if armed.is_some() && armed == released {
                    self.open_menu(&rows, armed.flatten(), column, row);
                }
                NavEffect::None
            }
            // The open menu consumes hover above, to light the entry under
            // the pointer. Nothing else follows the pointer yet (task 090).
            InputAction::Hover { .. } => NavEffect::None,
            InputAction::Rename => {
                self.begin_edit(&rows, Action::Rename);
                NavEffect::None
            }
            InputAction::NewFile => {
                self.begin_edit(&rows, Action::NewFile);
                NavEffect::None
            }
            InputAction::Delete => {
                self.begin_delete(&rows);
                NavEffect::None
            }
            InputAction::Undo => {
                self.undo();
                NavEffect::None
            }
            InputAction::Cut => {
                self.hold(&rows, false);
                NavEffect::None
            }
            InputAction::CopyEntries => {
                self.hold(&rows, true);
                NavEffect::None
            }
            InputAction::Paste => {
                self.paste(&rows);
                NavEffect::None
            }
            // The keyboard route in (task 092). Unlike a right-click it has no
            // pointer to reconcile with the selection, so it disturbs nothing:
            // it opens on whatever is already selected.
            InputAction::OpenMenu => {
                if let Some(point) = self.cursor_point(area(terminal), &rows)
                    && let Some(cursor) = self.view.selection.clone()
                {
                    self.open_menu(&rows, Some(cursor), point.0, point.1);
                }
                NavEffect::None
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
                    self.picked = vec![path];
                } else if self.perform_open(terminal, events, &path) {
                    return true;
                }
            }
        }
        self.finished(false)
    }

    /// The one open execution path — hotkeys, mouse, and the socket verb all
    /// land here (the action layer is shared by design).
    /// Returns true when a quit arrived while the child owned the terminal —
    /// a `ctl quit` or a signal must not be answered and then forgotten.
    fn perform_open(
        &mut self,
        terminal: &mut term::Term,
        events: &Receiver<AppEvent>,
        path: &Path,
    ) -> bool {
        match self.open_cmd.mode {
            OpenMode::Detached => {
                self.open_detached(path);
                false
            }
            OpenMode::Terminal => {
                self.open_in_terminal(terminal, path);
                // Events read before the handover completed are stale: apply
                // everything except old input. One drain helper, so this and
                // the loop's batch cannot diverge on what "handled" means —
                // and, as there, nothing runs behind a quit (ADR 0024).
                let stop = drain_batch(events, None, |pending| match pending {
                    AppEvent::Input(_) => BatchStep::Continue,
                    other => {
                        if self.handle(terminal, events, other) {
                            BatchStep::Quit
                        } else {
                            BatchStep::Continue
                        }
                    }
                });
                stop == BatchStop::Quit
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
            CtlEffect::Open(path) => self.perform_open(terminal, events, &path),
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
                Some(cursor) => {
                    let form = request.form.unwrap_or_default();
                    // `data` stays exactly what it has always been — one path,
                    // the cursor — and `paths` carries the whole selection in
                    // tree order beside it (task 074). Redefining `data` would
                    // not be additive evolution (ADR 0011).
                    let paths = self
                        .view
                        .selected(&self.rows())
                        .iter()
                        .map(|path| self.format_path(path, form))
                        .collect();
                    Response::ok_selection(self.format_path(&cursor, form), paths)
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
            Verb::SetOpenCmd => self.handle_set_open_cmd(&request),
            Verb::Quit => {
                effect = CtlEffect::Quit;
                Response::ok(None)
            }
        };
        (response, effect)
    }

    /// Replaces the open command (ADR 0035). The template is required; the
    /// display name and the mode are each left alone when absent, so a caller
    /// correcting one does not silently reset the others.
    fn handle_set_open_cmd(&mut self, request: &Request) -> Response {
        let Some(template) = request.value.as_deref() else {
            return Response::err("set-open-cmd needs a command template");
        };
        let mut cmd = match OpenCmd::from_template(template) {
            Ok(cmd) => cmd,
            Err(e) => return Response::err(e),
        };
        cmd.mode = match request.detached {
            Some(true) => OpenMode::Detached,
            Some(false) => OpenMode::Terminal,
            None => self.open_cmd.mode,
        };
        match request.name.as_deref() {
            Some(name) => cmd.set_name(name),
            // `from_template` starts nameless, so carrying the old name over
            // is what "absent leaves it alone" has to mean.
            None => {
                if let Some(existing) = self.open_cmd.name() {
                    cmd.set_name(existing);
                }
            }
        }
        self.open_cmd = cmd;
        Response::ok(None)
    }

    fn handle_set(&mut self, key: Option<SettingKey>, value: Option<&str>) -> Response {
        let (Some(key), Some(value)) = (key, value) else {
            return Response::err("set needs a setting and a value");
        };
        // Theme is a theme-id string, not the on/off SettingValue every other
        // key parses. The redraw at the end of the loop applies it live.
        // Numeric settings are parsed before SettingValue, as Theme is: their
        // value is not on/off/toggle.
        if let SettingKey::Badges = key {
            return match settings::parse_badges(value) {
                Ok(placement) => {
                    self.settings.badges = placement;
                    Response::ok(None)
                }
                Err(e) => Response::err(e),
            };
        }
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
            SettingKey::BranchBadges => {
                self.settings.branch_badges = value.apply(self.settings.branch_badges);
            }
            SettingKey::Scrollbar => {
                self.settings.scrollbar = value.apply(self.settings.scrollbar);
            }
            SettingKey::Theme | SettingKey::ScrollLines | SettingKey::Badges => {
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

    /// Arms a press on the hit row, or on nothing when the press landed
    /// outside the tree. Stores the row's real path, so a snapshot arriving
    /// before the release cannot redirect the click (ADR 0025).
    fn arm_press(&mut self, rows: &[Row], hit: Option<(usize, bool)>) {
        self.armed_press = hit
            .and_then(|(idx, on_chevron)| rows.get(idx).map(|row| (row.path.clone(), on_chevron)));
    }

    /// Completes an armed press, or abandons it. A click acts only when the
    /// release lands on the same row *and* the same zone as the press; every
    /// other outcome does nothing and clears a pending double-click, which is
    /// how ADR 0015 already treats an intervening click (ADR 0025).
    #[allow(clippy::doc_markdown)]
    fn resolve_release(
        &mut self,
        rows: &[Row],
        hit: Option<(usize, bool)>,
        modifier: input::ClickModifier,
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
        // A modified click is a selection gesture and nothing else: it never
        // activates, so it disarms any pending double-click. Whether the
        // modifier arrives at all is the terminal's decision — many keep
        // `Shift` for their own text selection (task 074).
        if self.selection_is_plural() && modifier != input::ClickModifier::None {
            self.click_timer.disarm();
            match modifier {
                input::ClickModifier::Extend => self.view.extend_to(rows, idx),
                input::ClickModifier::Toggle => self.view.toggle_selected(rows, idx),
                input::ClickModifier::None => {}
            }
            return NavEffect::None;
        }
        self.resolve_click(rows, idx, on_chevron, now)
    }

    /// Whether more than one row may be selected at once: always in the tree,
    /// and in the picker only under `--multi`, where a plural selection has
    /// somewhere to go (task 074).
    fn selection_is_plural(&self) -> bool {
        !self.mode.is_pick() || self.multi
    }

    /// Click decision (ADR 0015): a chevron click toggles (each completed
    /// click is its own toggle, and it disarms a pending double — chevron-
    /// then-name fast is a select); name clicks select, and only a completed
    /// double-click activates. Reached from the *release* (ADR 0025), never
    /// from the press. Tree semantics apply in both modes, since the picker
    /// renders the same tree (ADR 0023).
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
                    // A picking click also moves the selection there — and
                    // collapses it, so a click always confirms what it lands on.
                    self.view.focus(row.path.clone());
                }
                // Nothing selected is ever dropped silently: a row the filter
                // does not confirm refuses the whole pick and says which one.
                if let Some(unpickable) = rows
                    .iter()
                    .find(|row| self.view.is_selected(&row.path) && !row.pickable)
                {
                    return NavEffect::Message(format!(
                        "{} does not match the filter",
                        unpickable.name
                    ));
                }
                self.picked = self.view.selected(rows);
                return NavEffect::None;
            }
        }
        match click {
            Some((idx, on_chevron)) => self.view.on_click(&mut self.tree, rows, idx, on_chevron),
            None => self.view.on_enter(&mut self.tree, rows),
        }
    }

    /// One path in the form the request asked for.
    fn format_path(&self, path: &Path, form: PathForm) -> String {
        match form {
            PathForm::Name => path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            PathForm::Rel => {
                let rel = path.strip_prefix(&self.root).unwrap_or(path);
                if rel.as_os_str().is_empty() {
                    ".".into() // the root itself
                } else {
                    rel.display().to_string()
                }
            }
            PathForm::Abs => path.display().to_string(),
        }
    }

    // ---- the context menu (071) and the action layer (ADR 0029) ----

    /// The selection as the action layer sees it: paths in tree order, reduced
    /// to the facts an availability rule uses.
    fn action_targets(&self, rows: &[Row]) -> Vec<ActionTarget> {
        rows.iter()
            .filter(|row| self.view.is_selected(&row.path))
            .map(|row| ActionTarget {
                path: row.path.clone(),
                is_dir: row.kind.is_dir(),
                expanded: row.expanded,
                missing: row.missing,
            })
            .collect()
    }

    /// What a right-click at this point would target: `Some(Some(path))` for a
    /// row, `Some(None)` for the empty space below the last row (which targets
    /// the root), and `None` for anywhere the menu has no business opening —
    /// the scrollbar column, the status line, a dimmed row, or a row that is
    /// tracked in git but gone from disk. Keyed on the row rather than the
    /// cell, so a press and release a column apart still agree (ADR 0025).
    fn right_target(
        &self,
        area: Rect,
        rows: &[Row],
        column: u16,
        row: u16,
    ) -> Option<Option<PathBuf>> {
        // A menu that cannot be drawn must not open: the app would be modal
        // with nothing on screen.
        if area.width < 3 || area.height < 3 {
            return None;
        }
        let chrome = render::Chrome {
            settings: &self.settings,
            theme: &Theme::for_id(self.settings.theme),
            branch_axis: self.branch_axis(),
        };
        match render::hit_test(rows, &self.view, chrome, area, column, row) {
            Some((idx, _)) => {
                let hit_row = rows.get(idx)?;
                // A dimmed row is inert by every route (ADR 0023), and nothing
                // can act on a row that is not there.
                (hit_row.live && !hit_row.missing).then(|| Some(hit_row.path.clone()))
            }
            // `hit_test` says `None` for the empty space below the last row and
            // for inert furniture alike; only the former targets the root.
            None => render::on_tree_surface(rows.len(), chrome, area, column, row).then_some(None),
        }
    }

    /// The action layer's view of right now: what is selected, which mode, and
    /// what `Open` would actually run. Built in one place so a new fact is
    /// added once rather than at every call site.
    fn action_context<'a>(
        &'a self,
        targets: &'a [ActionTarget],
        program: &'a str,
    ) -> ActionContext<'a> {
        ActionContext {
            targets,
            picker: self.mode.is_pick(),
            // The whole command, not just its program: `open -a Preview` runs
            // `open` without being the platform handler.
            open_is_platform: self.open_cmd.is_platform_opener(),
            open_program: program,
            system_program: OpenCmd::platform_program(),
            undo: self.undo_label.as_deref(),
            pending: self.pending_label.as_deref(),
            // `Open` acts on the cursor, so the row that names what `Enter`
            // will do is the cursor's, not the selection's first.
            cursor: self
                .view
                .selection
                .as_deref()
                .and_then(|cursor| targets.iter().find(|target| target.path == cursor)),
        }
    }

    /// Opens the menu on a right-click. The selection is only disturbed when it
    /// has to be: a click inside it acts on all of it, a click outside collapses
    /// to that row first, and a click below the last row collapses to the root.
    /// Where a keyboard-opened menu should appear: the cursor row's own line,
    /// at the column its text starts on. Column zero would hang the menu off
    /// the far left of a deep row, reading as part of the tree rather than as
    /// something attached to the selection. `None` when the cursor is not on
    /// screen or nothing can hold the selection at all.
    fn cursor_point(&self, area: Rect, rows: &[Row]) -> Option<(u16, u16)> {
        let cursor = self.view.selection.as_deref()?;
        let index = rows.iter().position(|row| row.path == cursor)?;
        let viewport = render::tree_viewport_height(area);
        let offset = index.checked_sub(self.view.scroll)?;
        if offset >= viewport {
            return None;
        }
        let indent = rows[index].depth as u16 * render::INDENT_WIDTH;
        Some((
            area.x + indent.min(area.width.saturating_sub(1)),
            area.y + offset as u16,
        ))
    }

    /// Returns true when a menu opened, so the caller can turn motion tracking
    /// on. Terminal-free: opening a menu is a decision about rows, and only the
    /// tracking is I/O. `right_target` has already refused the points where a
    /// menu has no business appearing.
    fn open_menu(&mut self, rows: &[Row], target: Option<PathBuf>, column: u16, row: u16) -> bool {
        match target {
            Some(path) => {
                let Some(idx) = rows.iter().position(|r| r.path == path) else {
                    return false;
                };
                self.view.on_single_click_if_unselected(rows, idx);
            }
            // Below the last row: the root becomes the target, which is what
            // makes "new file in the root" reachable with nothing useful
            // selected.
            None => {
                if let Some(root_idx) = rows.iter().position(|r| r.path == self.root)
                    && rows[root_idx].live
                {
                    self.view.on_single_click(rows, root_idx);
                } else {
                    return false;
                }
            }
        }
        let rows = self.rows();
        let targets = self.action_targets(&rows);
        let context = self.action_context(&targets, self.open_cmd.display_name());
        let Some(menu) = Menu::open(context, column, row) else {
            return false;
        };
        self.menu = Some(menu);
        true
    }

    /// Input while the menu is open. Returns true when birch should quit.
    /// A thin wrapper: `menu_input` decides, and only the I/O happens here.
    fn handle_menu_input(
        &mut self,
        terminal: &mut term::Term,
        events: &Receiver<AppEvent>,
        action: InputAction,
    ) -> bool {
        match self.menu_input(area(terminal), action) {
            MenuStep::Keep => false,
            MenuStep::Close => {
                self.menu = None;
                false
            }
            MenuStep::Search(c) => {
                self.menu = None;
                self.search_push(c);
                false
            }
            MenuStep::Reopen(target) => {
                self.menu = None;
                self.armed_right = target;
                false
            }
            MenuStep::Invoke(action) => {
                self.menu = None;
                let quit = match self.perform(action) {
                    Performed::Open(path) => self.perform_open(terminal, events, &path),
                    Performed::OpenWith(path, cmd) => {
                        let configured = std::mem::replace(&mut self.open_cmd, cmd);
                        let quit = self.perform_open(terminal, events, &path);
                        self.open_cmd = configured;
                        quit
                    }
                    Performed::Write(sequence) => {
                        term::write_raw(terminal, sequence.as_bytes());
                        false
                    }
                    Performed::Nothing => false,
                };
                // The same terminator every other input path falls through to:
                // a pick made from the menu ends the picker there, not on
                // whatever key happens to arrive next.
                self.finished(quit)
            }
        }
    }

    /// What one input does to an open menu. Terminal-free, so every branch is
    /// reachable from a test: the wrapper above turns the outcome into I/O.
    fn menu_input(&mut self, area: Rect, action: InputAction) -> MenuStep {
        let layout = {
            let Some(menu) = self.menu.as_mut() else {
                return MenuStep::Keep;
            };
            menu.layout(area.x, area.y, area.width, area.height)
        };
        match action {
            InputAction::Esc => MenuStep::Close,
            // Typing is always search (ADR 0008), with no exception for a menu:
            // a layer that redefines the keyboard is how a modal language
            // starts. The menu closes and the character reaches the query.
            InputAction::Char(c) => MenuStep::Search(c),
            InputAction::Up => {
                menu_mut(&mut self.menu).step(false);
                MenuStep::Keep
            }
            InputAction::Down => {
                menu_mut(&mut self.menu).step(true);
                MenuStep::Keep
            }
            InputAction::Home => {
                menu_mut(&mut self.menu).go_to_edge(true);
                MenuStep::Keep
            }
            InputAction::End => {
                menu_mut(&mut self.menu).go_to_edge(false);
                MenuStep::Keep
            }
            InputAction::Enter => match menu_ref(&self.menu).selected_action() {
                Some(action) => MenuStep::Invoke(action),
                None => MenuStep::Keep,
            },
            InputAction::Hover { column, row } => {
                if let Some(index) = menu_ref(&self.menu).hit(layout, column, row) {
                    menu_mut(&mut self.menu).highlight(index);
                }
                MenuStep::Keep
            }
            InputAction::Press { column, row } => {
                // A press outside closes on the spot: there is no armed-click
                // dance to run for dismissal.
                if menu_ref(&self.menu).covers(layout, column, row) {
                    MenuStep::Keep
                } else {
                    MenuStep::Close
                }
            }
            // A right-click elsewhere moves the menu rather than leaving it
            // stranded: the old one closes, and the release opens a new one on
            // the row the press landed on.
            InputAction::RightPress { column, row } => {
                if menu_ref(&self.menu).covers(layout, column, row) {
                    return MenuStep::Keep;
                }
                let rows = self.rows();
                MenuStep::Reopen(self.right_target(area, &rows, column, row))
            }
            InputAction::Release { column, row, .. } => {
                // Only an entry invokes. `highlight` ignores separators by
                // design, so asking it to move and then reading the highlight
                // would fire whatever was standing there.
                let Some(index) = menu_ref(&self.menu).hit(layout, column, row) else {
                    return MenuStep::Keep;
                };
                match menu_ref(&self.menu).action_at(index) {
                    Some(action) => {
                        menu_mut(&mut self.menu).highlight(index);
                        MenuStep::Invoke(action)
                    }
                    None => MenuStep::Keep,
                }
            }
            // Everything else — scrolling, resizes, movement keys the menu does
            // not bind — leaves it alone.
            _ => MenuStep::Keep,
        }
    }

    /// The one way an action is performed. Hotkeys, the mouse, the menu, and
    /// the socket all arrive here; nothing reaches an operation another way
    /// (ADR 0029). Terminal-free, so it stays a decision about the tree: a file
    /// that needs an editor comes back for the caller to open.
    fn perform(&mut self, action: Action) -> Performed {
        let rows = self.rows();
        let targets = self.action_targets(&rows);
        if !action.available(self.action_context(&targets, self.open_cmd.display_name())) {
            return Performed::Nothing;
        }
        if let Some(form) = action.copy_form() {
            return self.copy_selection(&targets, form);
        }
        match action {
            Action::Open => {
                let effect = self.activate(&rows, None);
                self.expansion_dirty = true;
                match effect {
                    // In the picker `activate` has already confirmed, so this
                    // only ever carries a file the tree wants opened.
                    NavEffect::Open(path) => return Performed::Open(path),
                    NavEffect::Message(message) => self.status = message,
                    NavEffect::RequestExpand(path) => {
                        let _ = self.source_cmds.send(SourceCmd::Expand(path));
                    }
                    NavEffect::None => {}
                }
            }
            Action::RevealRootHere => {
                if let Some(target) = targets.first() {
                    let response = self.set_root(target.path.clone());
                    if !response.ok {
                        self.status = response.error.unwrap_or_default();
                    }
                }
            }
            // The desktop's own handler, detached: it hands off to another
            // application rather than taking the terminal, so `perform_open`
            // does the same thing it does for `--open-detached`.
            Action::OpenSystem => {
                if let Some(target) = targets.first() {
                    return Performed::OpenWith(target.path.clone(), OpenCmd::platform_opener());
                }
            }
            Action::Rename | Action::NewFile | Action::NewFolder => {
                let rows = self.rows();
                self.begin_edit(&rows, action);
            }
            Action::Delete => {
                let rows = self.rows();
                self.begin_delete(&rows);
            }
            Action::Undo => self.undo(),
            Action::Cut | Action::CopyEntries => {
                let rows = self.rows();
                self.hold(&rows, action == Action::CopyEntries);
            }
            Action::Paste => {
                let rows = self.rows();
                self.paste(&rows);
            }
            // Detached like `OpenSystem`: it hands the path to another
            // application rather than taking the terminal.
            Action::Reveal => {
                if let Some(target) = targets.first() {
                    return Performed::OpenWith(
                        OpenCmd::reveal_target(&target.path),
                        OpenCmd::reveal(),
                    );
                }
            }
            // Handled above, before the match: every copy shares one path.
            Action::CopyName | Action::CopyRelativePath | Action::CopyAbsolutePath => {}
        }
        Performed::Nothing
    }

    /// Resolves the selection to text and hands it to the clipboard. The
    /// transport is chosen per copy from the environment (`birch_core::
    /// clipboard`); OSC 52 comes back as a sequence for the caller to write,
    /// because the terminal belongs to the render layer.
    fn copy_selection(&mut self, targets: &[ActionTarget], form: CopyForm) -> Performed {
        let path_form = match form {
            CopyForm::Name => PathForm::Name,
            CopyForm::Relative => PathForm::Rel,
            CopyForm::Absolute => PathForm::Abs,
        };
        let text = targets
            .iter()
            .map(|target| self.format_path(&target.path, path_form))
            .collect::<Vec<_>>()
            .join("\n");
        let transport = clipboard::choose(clipboard::is_remote(), &clipboard::available_tools());
        match clipboard::copy(&text, transport) {
            Ok(sequence) => {
                self.status = copied_message(targets.len(), form);
                match sequence {
                    Some(sequence) => Performed::Write(sequence),
                    None => Performed::Nothing,
                }
            }
            Err(e) => {
                self.status = format!("could not copy: {e}");
                Performed::Nothing
            }
        }
    }

    /// Paints a `Shift` extension once its match reveal has converged. Match
    /// stepping lands over several frames — ancestors have to be expanded and
    /// listed first — so the range cannot be drawn where the key was pressed.
    fn settle_extension(&mut self) {
        if self.pending_extend && self.pending_reveal.is_none() {
            let rows = self.rows();
            self.view.finish_extending(&rows);
            self.pending_extend = false;
        }
    }

    /// The terminator every input path shares: birch stops when an action asked
    /// it to, or when a pick has been made. The menu path returning early is
    /// exactly what once left a pick sitting until the next keystroke.
    fn finished(&self, quit: bool) -> bool {
        quit || !self.picked.is_empty()
    }

    // ---- the inline editor and the delete confirmation (029) ----

    /// Opens an editor over the cursor row (rename) or as a phantom under the
    /// target directory (create). Nothing touches the tree until `Enter`.
    fn begin_edit(&mut self, rows: &[Row], action: Action) {
        let targets = self.action_targets(rows);
        if !action.available(self.action_context(&targets, self.open_cmd.display_name())) {
            return;
        }
        let Some(target) = targets.first() else {
            return;
        };
        self.edit = Some(match action {
            Action::Rename => {
                // The *tail's* name, not the row's label: a compacted chain
                // renders as `a/b/c` while its path is the tail, and pre-filling
                // the label would resolve against the tail's parent and move the
                // directory somewhere nobody asked for.
                let name = target
                    .path
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
                let kind = if target.is_dir {
                    NodeKind::Dir
                } else {
                    NodeKind::File
                };
                Editor::rename(target.path.clone(), &name, kind)
            }
            // A create needs a directory: a selected file names its parent.
            _ => Editor::create(
                if target.is_dir {
                    target.path.clone()
                } else {
                    target.path.parent().unwrap_or(&self.root).to_path_buf()
                },
                action == Action::NewFolder,
            ),
        });
    }

    /// Input while an editor is open. Printables reach the field rather than
    /// the search, which is ADR 0008 read as it was always meant: they go to
    /// whatever text field is focused, and an edit focuses a different one.
    fn handle_edit_input(&mut self, action: InputAction) {
        match action {
            InputAction::Esc => self.edit = None,
            InputAction::Enter => {
                if let Some(editor) = self.edit.take() {
                    // A refused name keeps the field open with the text intact.
                    // The usual refusal is a clash, and the fix is a few
                    // keystrokes — closing the field would throw the name away
                    // and, with the tree unchanged behind it, read as though
                    // the create had silently overwritten something.
                    if !self.commit_edit(&editor) {
                        self.edit = Some(editor);
                    }
                }
            }
            editing => {
                let Some(editor) = self.edit.as_mut() else {
                    return;
                };
                match editing {
                    InputAction::Char(c) => editor.insert(c),
                    InputAction::Backspace => editor.backspace(),
                    InputAction::Delete => editor.delete(),
                    InputAction::Left => editor.left(),
                    InputAction::Right => editor.right(),
                    InputAction::Home => editor.home(),
                    InputAction::End => editor.end(),
                    // Everything else leaves the field alone rather than acting
                    // on the tree behind it.
                    _ => {}
                }
            }
        }
    }

    /// Returns whether the edit went through. A `false` keeps the field open.
    fn commit_edit(&mut self, editor: &Editor) -> bool {
        let input = editor.buffer();
        if input.trim().is_empty() {
            // Every other failure path says something; silence here reads as a
            // field that swallowed the edit.
            self.status = "a name is required".into();
            return false;
        }
        let result = match editor.kind() {
            EditKind::Rename { path, .. } => ops::rename(path, input),
            EditKind::Create { parent, directory } => ops::create(parent, input, *directory),
        };
        match result {
            Ok(deltas) => {
                // Recorded from the deltas rather than re-derived: `create`
                // announces the *shallowest* path it made, which is what undo
                // has to take back — `a/b/c.txt` typed into an empty tree
                // created `a`, and removing only `c.txt` would leave the rest.
                let landed = deltas.iter().find_map(added_path);
                match (editor.kind(), landed) {
                    (EditKind::Rename { path, .. }, Some(to)) => {
                        self.history.push(undo::Step::Moved {
                            pairs: vec![(path.clone(), to)],
                        });
                    }
                    (EditKind::Create { .. }, Some(path)) => {
                        self.history.push(undo::Step::Created { paths: vec![path] });
                    }
                    _ => {}
                }
                for delta in deltas {
                    self.tree.apply(delta);
                }
                self.expansion_dirty = true;
                self.status.clear();
                self.refresh_undo_label();
                true
            }
            Err(e) => {
                self.status = e.to_string();
                false
            }
        }
    }

    /// Holds the selection for a paste. Replaces whatever was held: a
    /// clipboard with one slot is the only kind anybody can keep track of.
    fn hold(&mut self, rows: &[Row], duplicate: bool) {
        let targets = self.action_targets(rows);
        let action = if duplicate {
            Action::CopyEntries
        } else {
            Action::Cut
        };
        if !action.available(self.action_context(&targets, self.open_cmd.display_name())) {
            return;
        }
        let paths: Vec<PathBuf> = targets.iter().map(|t| t.path.clone()).collect();
        if paths.is_empty() {
            return;
        }
        let what = describe(&paths);
        self.status = format!(
            "{} {what} — ^V pastes",
            if duplicate { "Copied" } else { "Cut" }
        );
        self.pending_label = Some(what);
        self.pending = Some(Pending { paths, duplicate });
    }

    /// Puts what is held into the selected directory — the selection itself if
    /// it is one, otherwise its parent, which is where a create goes too.
    fn paste(&mut self, rows: &[Row]) {
        let targets = self.action_targets(rows);
        if !Action::Paste.available(self.action_context(&targets, self.open_cmd.display_name())) {
            return;
        }
        let Some(target) = targets.first() else {
            return;
        };
        let directory = if target.is_dir {
            target.path.clone()
        } else {
            target.path.parent().unwrap_or(&self.root).to_path_buf()
        };
        let Some(held) = self.pending.take() else {
            return;
        };
        self.pending_label = None;

        let what = describe(&held.paths);
        let mut moved = Vec::new();
        let mut created = Vec::new();
        for path in &held.paths {
            let outcome = if held.duplicate {
                ops::copy_into(path, &directory)
            } else {
                ops::move_into(path, &directory)
            };
            match outcome {
                Ok(deltas) => {
                    if let Some(to) = deltas.iter().find_map(added_path) {
                        if held.duplicate {
                            created.push(to);
                        } else {
                            moved.push((path.clone(), to));
                        }
                    }
                    for delta in deltas {
                        self.tree.apply(delta);
                    }
                }
                Err(e) => {
                    // Whatever already landed is recorded before reporting, so
                    // a paste that stops halfway is still one `Ctrl-Z`.
                    self.finish_paste(moved, created);
                    self.status = e.to_string();
                    return;
                }
            }
        }
        // Nothing landed and nothing failed: every path was already here. An
        // operation that did nothing must not report that it did something.
        if moved.is_empty() && created.is_empty() {
            self.status = format!(
                "{what} is already in {}",
                self.format_path(&directory, PathForm::Rel)
            );
            return;
        }
        let duplicate = held.duplicate;
        self.finish_paste(moved, created);
        self.expansion_dirty = true;
        self.status = format!(
            "{} {what} \u{2192} {}",
            if duplicate { "Copied" } else { "Moved" },
            self.format_path(&directory, PathForm::Rel)
        );
    }

    fn finish_paste(&mut self, moved: Vec<(PathBuf, PathBuf)>, created: Vec<PathBuf>) {
        self.remember_moves(moved);
        if !created.is_empty() {
            self.history.push(undo::Step::Created { paths: created });
            self.refresh_undo_label();
        }
    }

    /// Records a batch of moves as one undo step, so a drag that carried five
    /// rows is taken back in one `Ctrl-Z` rather than five.
    fn remember_moves(&mut self, pairs: Vec<(PathBuf, PathBuf)>) {
        if pairs.is_empty() {
            return;
        }
        self.history.push(undo::Step::Moved { pairs });
        self.refresh_undo_label();
    }

    /// Records a delete. It cannot be reversed — the trash's restore is not
    /// available on every platform birch runs on — but it is still recorded, so
    /// that `Ctrl-Z` after a delete says where the files went instead of
    /// silently reversing whatever happened *before* it.
    fn remember_removal(&mut self, what: &str, trashed: Vec<birch_core::disposal::Trashed>) {
        if trashed.is_empty() {
            return;
        }
        self.history.push(undo::Step::Removed {
            what: what.into(),
            trashed,
        });
        self.refresh_undo_label();
    }

    fn refresh_undo_label(&mut self) {
        self.undo_label = self.history.next().map(undo::Step::describe);
    }

    fn undo(&mut self) {
        let Some(undone) = self.history.undo() else {
            return;
        };
        for delta in undone.deltas {
            self.tree.apply(delta);
        }
        self.expansion_dirty = true;
        self.status = undone.message;
        self.refresh_undo_label();
    }

    /// How loudly the bottom line is drawn: a message is something the user did
    /// not have on screen a moment ago, and outranks the chrome it replaces.
    fn bottom_kind(&self) -> render::BottomKind {
        if self.status.is_empty() {
            render::BottomKind::Idle
        } else {
            render::BottomKind::Message
        }
    }

    /// Starts a delete. Tracked files go straight to the trash — recoverable
    /// twice over, and a prompt there is theatre. Anything untracked asks
    /// first, because it exists nowhere else and a trash can be emptied
    /// (ADR 0031).
    fn begin_delete(&mut self, rows: &[Row]) {
        let targets = self.action_targets(rows);
        if !Action::Delete.available(self.action_context(&targets, self.open_cmd.display_name())) {
            return;
        }
        self.trash_now(targets.iter().map(|t| t.path.clone()).collect());
    }

    fn trash_now(&mut self, paths: Vec<PathBuf>) {
        let mut removal = ops::trash(&paths);
        let trashed = std::mem::take(&mut removal.trashed);
        let removed = self.apply_removal(&removal);
        let Some(failure) = removal.failed else {
            let what = describe(&paths);
            self.remember_removal(&what, trashed);
            self.status = format!("Moved {what} to the trash");
            return;
        };
        // Whatever already went is still restorable, so it is recorded even
        // though the batch failed — the rows are gone from the tree either way.
        self.remember_removal(&describe(&removed), trashed);
        // Whatever already went is gone; only the rest is still a question.
        // Asking about paths that are already in the trash would be a prompt
        // about nothing, and confirming it would delete nothing.
        let remaining: Vec<PathBuf> = paths
            .into_iter()
            .filter(|path| !removed.contains(path))
            .collect();
        if remaining.is_empty() {
            self.status = failure.to_string();
            return;
        }
        // The delete ends here. birch has no removal that bypasses the trash
        // (ADR 0031), so there is nothing to fall back to and nothing to ask —
        // only the failure to report, and what already went, since the tree no
        // longer shows those rows.
        let moved = if removed.is_empty() {
            String::new()
        } else {
            format!("{} went. ", describe(&removed))
        };
        self.status = format!("{moved}{} not deleted — {failure}", describe(&remaining));
    }

    /// Applies a removal's deltas and returns the paths that actually went, so
    /// a caller can tell what is left to ask about.
    fn apply_removal(&mut self, removal: &ops::Removal) -> Vec<PathBuf> {
        let mut removed = Vec::with_capacity(removal.deltas.len());
        for delta in &removal.deltas {
            if let TreeDelta::Removed { path } = delta {
                removed.push(path.clone());
            }
            self.tree.apply(delta.clone());
        }
        if !removed.is_empty() {
            self.expansion_dirty = true;
        }
        removed
    }

    // ---- dragging a selection into a directory (072) ----

    /// Called on every drag event. Promotes an armed press into a drag once the
    /// pointer leaves the row it started on, then keeps the drop target current.
    fn update_drag(&mut self, area: Rect, rows: &[Row], column: u16, row: u16) {
        // A drag moves files, and mutations are human-initiated and disabled in
        // the picker (ADR 0032). The gesture bypasses `Action::available`, so
        // the rule has to be stated here too.
        if self.mode.is_pick() {
            return;
        }
        if self.drag.is_none() {
            let Some((path, _)) = self.armed_press.clone() else {
                return;
            };
            let Some(origin) = rows.iter().position(|r| r.path == path) else {
                return;
            };
            // Still on the row the press began on: this is a click that
            // wobbled, which ADR 0025 already treats as a click.
            if self.row_at(area, rows, row) == Some(origin) {
                return;
            }
            // The dragged set follows the menu's targeting rule: a press inside
            // the selection drags all of it, outside it drags that row alone.
            if !self.view.is_selected(&path) {
                self.view.on_single_click(rows, origin);
            }
            let paths = self.view.selected(rows);
            if paths.is_empty() {
                return;
            }
            self.armed_press = None;
            self.click_timer.disarm();
            self.drag = Some(Drag {
                paths,
                target: None,
            });
        }
        let target = self.drop_target(area, rows, column, row);
        self.spring_load(rows, target.as_ref());
        self.scroll_at_edge(area, rows, row);
        if let Some(drag) = self.drag.as_mut() {
            drag.target = target;
        }
        self.status = self.drag_message();
    }

    /// Hovering a collapsed directory during a drag opens it. This needs no
    /// carve-out from "never auto-expand": that rule is scoped to **ignored**
    /// directories, and search already auto-expands. An ignored directory stays
    /// shut while remaining a legal target — it simply does not open itself to
    /// show you (ADR 0032).
    fn spring_load(&mut self, rows: &[Row], target: Option<&DropTarget>) {
        let Some(target) = target else { return };
        let Some(row) = rows.iter().find(|row| row.path == target.directory) else {
            return;
        };
        if !row.kind.is_dir() || row.expanded || row.ignored || row.missing {
            return;
        }
        let (path, loaded) = (row.path.clone(), row.loaded);
        self.tree.set_expanded(&path, true);
        self.expansion_dirty = true;
        if !loaded && self.requested_peeks.insert(path.clone()) {
            let _ = self.source_cmds.send(SourceCmd::Expand(path));
        }
    }

    /// Dragging into the top or bottom row scrolls by one. Holding the pointer
    /// still does not keep scrolling — motion events only arrive when the
    /// pointer moves — which is the documented limit rather than a timer this
    /// event loop does not otherwise need (ADR 0032).
    fn scroll_at_edge(&mut self, area: Rect, rows: &[Row], row: u16) {
        let viewport = render::tree_viewport_height(area);
        if viewport == 0 {
            return;
        }
        let Some(offset) = row.checked_sub(area.y) else {
            return;
        };
        let delta = if offset == 0 {
            -1
        } else if offset as usize + 1 >= viewport {
            1
        } else {
            return;
        };
        let before = self.view.scroll;
        self.view.scroll_by(rows.len(), delta, viewport);
        if self.view.scroll != before {
            self.rows_len = rows.len();
        }
    }

    /// Which row a screen line falls on, if any.
    fn row_at(&self, area: Rect, rows: &[Row], row: u16) -> Option<usize> {
        let viewport = render::tree_viewport_height(area);
        let offset = row.checked_sub(area.y)? as usize;
        if offset >= viewport {
            return None;
        }
        let index = self.view.scroll + offset;
        (index < rows.len()).then_some(index)
    }

    /// Where a drop would land, and why it would be refused. A directory takes
    /// the drop; a file offers its parent; below the rows is the root; a dimmed
    /// row is inert, as everywhere (ADR 0023).
    fn drop_target(&self, area: Rect, rows: &[Row], column: u16, row: u16) -> Option<DropTarget> {
        let chrome = render::Chrome {
            settings: &self.settings,
            theme: &Theme::for_id(self.settings.theme),
            branch_axis: self.branch_axis(),
        };
        let directory = match render::hit_test(rows, &self.view, chrome, area, column, row) {
            Some((index, _)) => {
                let hit = rows.get(index)?;
                if !hit.live {
                    return None;
                }
                if hit.kind.is_dir() && !hit.missing {
                    hit.path.clone()
                } else {
                    hit.path.parent()?.to_path_buf()
                }
            }
            None => {
                if !render::on_tree_surface(rows.len(), chrome, area, column, row) {
                    return None;
                }
                self.root.clone()
            }
        };
        let refusal = self.refusal(&directory);
        Some(DropTarget { directory, refusal })
    }

    /// Why this directory cannot take the drop, visible before the button is
    /// released rather than discovered after it (ADR 0032).
    fn refusal(&self, directory: &Path) -> Option<String> {
        let paths = self
            .drag
            .as_ref()
            .map(|d| d.paths.as_slice())
            .unwrap_or(&[]);
        let mut landing: HashSet<&std::ffi::OsStr> = HashSet::new();
        for path in paths {
            if directory.starts_with(path) {
                return Some("cannot move a folder inside itself".into());
            }
            if path.parent() == Some(directory) {
                return Some("already there".into());
            }
            let Some(name) = path.file_name() else {
                continue;
            };
            // Two dragged rows sharing a basename collide with *each other*,
            // which the filesystem cannot yet show — a refusal has to reason
            // about where the whole set would land, not only where it is now.
            if !landing.insert(name) {
                return Some(format!(
                    "two selected rows are named {}",
                    name.to_string_lossy()
                ));
            }
            // Not `exists()`: it follows links, so a dangling one would read as
            // free and be replaced.
            if std::fs::symlink_metadata(directory.join(name)).is_ok() {
                return Some(format!("{} already exists there", name.to_string_lossy()));
            }
        }
        None
    }

    fn drag_message(&self) -> String {
        let Some(drag) = &self.drag else {
            return String::new();
        };
        let what = describe(&drag.paths);
        match &drag.target {
            Some(DropTarget {
                refusal: Some(why), ..
            }) => format!("Moving {what} — {why}"),
            Some(DropTarget { directory, .. }) => {
                format!(
                    "Moving {what} \u{2192} {}",
                    self.format_path(directory, PathForm::Rel)
                )
            }
            None => format!("Moving {what}"),
        }
    }

    /// Completes the drag. A refused target does nothing and says so; the move
    /// itself has no undo, so the status line reports where things went.
    fn finish_drag(&mut self) {
        let Some(drag) = self.drag.take() else {
            return;
        };
        let Some(target) = drag.target else {
            self.status.clear();
            return;
        };
        if let Some(why) = target.refusal {
            self.status = format!("Not moved — {why}");
            return;
        }
        let what = describe(&drag.paths);
        let mut moved = Vec::new();
        for path in &drag.paths {
            match ops::move_into(path, &target.directory) {
                Ok(deltas) => {
                    if let Some(to) = deltas.iter().find_map(added_path) {
                        moved.push((path.clone(), to));
                    }
                    for delta in deltas {
                        self.tree.apply(delta);
                    }
                }
                Err(e) => {
                    // Whatever already moved is recorded even though the batch
                    // failed: a half-applied drag is exactly the thing a user
                    // reaches for undo about.
                    self.remember_moves(moved);
                    self.status = e.to_string();
                    return;
                }
            }
        }
        self.remember_moves(moved);
        self.expansion_dirty = true;
        self.status = format!(
            "Moved {what} \u{2192} {}",
            self.format_path(&target.directory, PathForm::Rel)
        );
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
                saved_selection: self.view.snapshot_selection(),
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
            // abandoned query must not keep mutating the tree, and an extension
            // waiting on that reveal must not survive the narrowing that
            // defined which rows it could cover.
            self.search = None;
            self.pending_reveal = None;
            self.pending_extend = false;
            self.view.end_extending();
            return;
        }
        self.rematch();
    }

    /// Esc backs out one layer (ADR 0012): an active search clears (tree
    /// mode restores the pre-search view); with nothing to dismiss, Esc
    /// quits — the picker without a pick, the tree like `Ctrl-Q`.
    fn on_esc(&mut self) -> bool {
        // Esc backs out one layer at a time (ADR 0012); the menu is now the
        // outermost, so the order is menu → search → quit.
        match self.search.take() {
            Some(state) => {
                // Both modes restore the pre-search view now: the picker keeps
                // the tree, so it has a view worth putting back (ADR 0023).
                self.view.restore_selection(state.saved_selection);
                self.view.scroll = state.saved_scroll;
                self.pending_reveal = None;
                self.pending_extend = false;
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
        self.rematch_inner(true);
    }

    /// Recompute matches without moving the cursor. Used when the *corpus*
    /// grew rather than the query changing: a chunk arriving from the index
    /// walk may hold matches that sort before wherever the user has settled,
    /// and the cursor must not travel under a reader's eyes (ADR 0026).
    fn rematch_keeping_selection(&mut self) {
        self.rematch_inner(false);
    }

    fn rematch_inner(&mut self, may_reveal: bool) {
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
            && may_reveal
            && self.pending_reveal.is_none()
            && self.view.selection.as_deref() != Some(target.as_path())
        {
            self.reveal(target);
        }
    }

    /// How many matches sort before the current selection in tree order —
    /// `0..=matches.len()`, **un-wrapped**. A selection sitting past the last
    /// match yields `len`, which single stepping folds to the first match and
    /// page stepping clamps to the last (task 081); the two need to tell those
    /// apart, so the raw position lives here.
    fn match_position(&self, matches: &[Match]) -> usize {
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
        search::tree_order_position(matches, &rel, is_dir)
    }

    /// The forward anchor: the index of the first match at or after the current
    /// selection in tree order, wrapping to the first match when none follows.
    /// Falls back to the first match when there is no selection to anchor on.
    fn anchor_index(&self, matches: &[Match]) -> usize {
        let position = self.match_position(matches);
        // Past the last match, the anchor wraps to the first.
        if position == matches.len() {
            0
        } else {
            position
        }
    }

    /// Steps `delta` matches **relative to the selection**, not to a remembered
    /// pointer. `→`, `←`, and the mouse all move the selection without going
    /// through here, so a stored index goes stale the moment one of them is
    /// used — stepping would then swallow a keystroke or jump backwards.
    /// Deriving the position each time cannot desynchronise.
    ///
    /// `wrap` distinguishes the two callers: a single step off the end has an
    /// obvious destination and wraps, while a page-sized jump clamps — wrapping
    /// one would teleport across the whole result set without saying so
    /// (task 081). `Home`/`End` are a clamped step large enough to reach an edge.
    fn step_match(&mut self, delta: isize, wrap: bool) {
        let matches = match &self.search {
            Some(state) if !state.matches.is_empty() => state.matches.clone(),
            _ => return,
        };
        let n = matches.len() as isize;
        let position = self.match_position(&matches);
        let on_match = position < matches.len()
            && self.view.selection.as_deref() == Some(matches[position].entry.abs.as_path());
        // Off a match, `position` counts the matches *before* the selection, so
        // the next one forward already sits at `position` and the previous at
        // `position - 1`.
        let target = position as isize + delta - isize::from(delta > 0 && !on_match);
        let next = if wrap {
            target.rem_euclid(n)
        } else {
            target.clamp(0, n - 1)
        };
        self.reveal(matches[next as usize].entry.abs.clone());
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
        self.settle_extension();
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
        let bottom_kind = self.bottom_kind();
        let branch_axis = self.branch_axis();
        let theme = Theme::for_id(self.settings.theme);
        let area = area(terminal);
        // The menu is laid out here and drawn from the same result, so the
        // painted rows and the clickable ones cannot drift apart.
        let menu_layout = self
            .menu
            .as_mut()
            .map(|menu| menu.layout(area.x, area.y, area.width, area.height));
        let drop_target = self
            .drag
            .as_ref()
            .and_then(|drag| drag.target.as_ref())
            .filter(|target| target.refusal.is_none())
            .map(|target| target.directory.clone());
        let (view, settings, menu) = (&self.view, &self.settings, &self.menu);
        let chrome = render::Chrome {
            settings,
            theme: &theme,
            branch_axis,
        };
        let editor = &self.edit;
        let mut caret = None;
        terminal.draw(|frame| {
            render::draw(frame, &rows, view, chrome, &bottom, bottom_kind);
            // A refused target is drawn as no target at all: the absence of the
            // highlight is the answer, before the button is released.
            if let Some(directory) = &drop_target {
                render::draw_drop_target(frame, &rows, view, chrome, directory);
            }
            // The editor sits over the tree and under the menu; the two are
            // never open together, so the order only fixes a convention.
            if let Some(editor) = editor {
                caret = render::draw_edit(frame, &rows, view, chrome, editor);
            }
            if let (Some(menu), Some(layout)) = (menu, menu_layout) {
                render::draw_menu(frame, menu, layout, &theme);
            }
        })?;
        // A visible caret is what makes the edit self-evident, which is the
        // property that makes it an admissible mode at all (ADR 0030).
        match caret {
            Some((x, y)) => term::show_caret(terminal, x, y),
            None => term::hide_caret(terminal),
        }
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
            // Three index states, three different things to say (ADR 0026).
            // "no matches" is a claim about the corpus, so it waits until
            // there is a whole corpus to make it about.
            match (&self.index, n) {
                (None, _) => format!("{prompt} (indexing…)"),
                (Some(index), 0) if !index.is_complete() => {
                    format!("{prompt} (indexing…)")
                }
                (Some(_), 0) => format!("{prompt} (no matches)"),
                (Some(index), _) if !index.is_complete() => format!(
                    "{prompt} ({}/{n}, indexing…)",
                    self.anchor_index(&state.matches) + 1
                ),
                _ => format!("{prompt} ({}/{n})", self.anchor_index(&state.matches) + 1),
            }
        } else if self.mode.is_pick() {
            "> type to filter, Enter picks the selection, Esc quits".into()
        } else {
            // The root row carries the path annotation; the idle bottom line
            // stays clear for messages.
            String::new()
        };
        match (base.is_empty(), self.status.is_empty()) {
            (_, true) => base,
            // Nothing to hang the message off: an idle tree has no bottom line
            // of its own, and a leading dash reads as a truncated sentence.
            (true, false) => self.status.clone(),
            (false, false) => format!("{base} — {}", self.status),
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
            // A fetch moves the base the branch column compares against, and
            // touches nothing in the working tree (task 078).
            desired.insert(repo.join(".git/refs/remotes"));
            desired.insert(repo.join(".git/refs/remotes/origin"));
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
    /// separate from the rows so a wheel burst costs a clamp, not a tree walk
    /// (ADR 0024); the clamp itself stays in `FlatView`.
    fn scroll_rows(&mut self, delta: isize, viewport: usize) {
        self.view.scroll_by(self.rows_len, delta, viewport);
    }

    /// Whether the branch column is shown: the setting is on, git is on, a
    /// repo was found, and a base resolved. Without a base there is nothing to
    /// compare against, so the column is absent rather than blank (task 078).
    fn branch_axis(&self) -> bool {
        self.settings.branch_badges
            && self
                .git_state
                .as_deref()
                .is_some_and(|git| git.base().is_some())
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
        // stderr is piped rather than discarded so a failure can say *why*
        // (task 085): a host adapter's "no focused cmux window" is the whole
        // value of the channel, and an exit code alone is not one. stdout
        // stays null — it is not a failure channel, and a long-lived child
        // may never close it.
        let spawned = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn();
        match spawned {
            Ok(mut child) => {
                let events = self.open_events.clone();
                let name = program_name(program).to_string();
                thread::spawn(move || {
                    // Drained before the wait, or a chatty child fills the
                    // pipe and blocks forever on a write nobody is reading.
                    let reason = child.stderr.take().map(drain_reason).unwrap_or_default();
                    let Ok(status) = child.wait() else { return };
                    if !status.success() {
                        let _ = events.send(OpenEvent {
                            program: name,
                            code: status.code(),
                            reason,
                        });
                    }
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

/// The most of a child's stderr that reaches the status line. Wider than the
/// line itself on any sane terminal, so the cap only stops a runaway.
const REASON_LIMIT: usize = 120;

/// A program as invoked, without its directory — the status line names what
/// was run, not where it lives.
fn program_name(program: &str) -> &str {
    program
        .rsplit(std::path::MAIN_SEPARATOR)
        .next()
        .unwrap_or(program)
}

/// The child's first line of stderr, cleaned and bounded, with the rest read
/// and dropped so the child is never blocked on a full pipe.
fn drain_reason(mut stderr: impl io::Read) -> String {
    use io::Read as _;
    let mut buf = Vec::new();
    let _ = (&mut stderr).take(4096).read_to_end(&mut buf);
    let _ = io::copy(&mut stderr, &mut io::sink());
    String::from_utf8_lossy(&buf)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| {
            line.chars()
                .filter(|c| !c.is_control())
                .take(REASON_LIMIT)
                .collect()
        })
        .unwrap_or_default()
}

/// What a failed detached open says. The reason when the child gave one, the
/// exit status when it did not — the same wording terminal-mode opens use.
fn open_failure_message(failure: &OpenEvent) -> String {
    let OpenEvent {
        program,
        code,
        reason,
    } = failure;
    if !reason.is_empty() {
        return format!("open failed: {program}: {reason}");
    }
    match code {
        Some(code) => format!("open failed: {program} exited with {code}"),
        None => format!("open failed: {program} was killed"),
    }
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
        /// Detached-open failures the app would have reported (task 085).
        pub(super) open_rx: mpsc::Receiver<OpenEvent>,
    }

    pub(super) fn harness(mode: Mode) -> Harness {
        let (source_tx, source_rx) = mpsc::channel();
        let (watch_tx, watch_rx) = mpsc::channel();
        let (index_tx, index_rx) = mpsc::channel();
        let (git_tx, git_rx) = mpsc::channel();
        let (open_tx, open_rx) = mpsc::channel();
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
            open_events: open_tx,
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
            picked: Vec::new(),
            multi: false,
            input_paused: Arc::new(AtomicBool::new(false)),
            click_timer: input::ClickTimer::default(),
            menu: None,
            armed_right: None,
            edit: None,
            history: undo::History::default(),
            undo_label: None,
            pending: None,
            pending_label: None,
            drag: None,
            pending_extend: false,
            armed_press: None,
            last_rematch: None,
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
            open_rx,
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

    pub(super) fn index_of(entries: &[(&str, bool)]) -> Arc<SearchIndex> {
        Arc::new(SearchIndex::from_paths(Path::new("/r"), entries))
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

    /// Task 081: a page-sized step through the matches clamps where a single
    /// step wraps, so `PgDn` past the last match stops on it instead of
    /// teleporting to the first.
    #[test]
    fn a_page_of_matches_clamps_where_a_single_step_wraps() {
        let mut h = harness(Mode::Tree);
        let names = ["a1.txt", "a2.txt", "a3.txt", "a4.txt"];
        feed(&mut h.app, "/r", &names.map(|n| (n, NodeKind::File)));
        h.app.index = Some(index_of(&names.map(|n| (n, false))));
        h.app.search_push('a');
        assert_eq!(h.app.search.as_ref().unwrap().matches.len(), 4);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a1.txt"))
        );

        // A page of two lands two matches on.
        h.app.step_match(2, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a3.txt"))
        );
        // Past the end it stops on the last match rather than wrapping.
        h.app.step_match(2, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a4.txt"))
        );
        h.app.step_match(2, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a4.txt"))
        );
        // A single step from the last match still wraps.
        h.app.step_match(1, true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a1.txt"))
        );
        // Backwards clamps the same way, and End/Home are a step big enough to
        // reach an edge.
        h.app.step_match(-2, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a1.txt"))
        );
        h.app.step_match(4, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a4.txt"))
        );
        h.app.step_match(-4, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a1.txt"))
        );
    }

    /// The selection can sit *past* every match — `match_position` reports
    /// `len` there, which single stepping folds to the first match and page
    /// stepping must clamp to the last.
    #[test]
    fn stepping_from_past_the_last_match_wraps_but_paging_clamps() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a1.txt", NodeKind::File), ("zz.txt", NodeKind::File)],
        );
        h.app.index = Some(index_of(&[("a1.txt", false), ("zz.txt", false)]));
        h.app.search_push('a');
        assert_eq!(h.app.search.as_ref().unwrap().matches.len(), 1);
        // Park the selection after the only match.
        h.app.view.focus("/r/zz.txt".into());
        h.app.step_match(3, false);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/a1.txt"))
        );
    }

    /// A `Shift` step through the matches extends over them. The reveal lands
    /// over several frames, so the range is painted when the cursor settles —
    /// once, and never again, or rows that were dim when the range was made
    /// would join it as soon as the narrowing lifted.
    #[test]
    fn shift_stepping_the_matches_extends_over_them() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("a.txt", NodeKind::File),
                ("b.txt", NodeKind::File),
                ("c_a.txt", NodeKind::File),
            ],
        );
        h.app.index = Some(index_of(&[
            ("a.txt", false),
            ("b.txt", false),
            ("c_a.txt", false),
        ]));
        h.app.search_push('a');
        assert_eq!(h.app.search.as_ref().unwrap().matches.len(), 2);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/a.txt")));

        let rows = h.app.rows();
        h.app.view.begin_extending(&rows);
        h.app.pending_extend = true;
        h.app.step_match(1, true);
        h.app.settle_extension();

        let rows = h.app.rows();
        let selected: Vec<_> = h
            .app
            .view
            .selected(&rows)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        // b.txt sits between the two matches and is dim, so it is skipped.
        assert_eq!(selected, ["a.txt", "c_a.txt"]);

        // Backspacing the query away ends the search *and* the extension, so
        // b.txt does not join the range the moment it becomes live again.
        h.app.search_pop();
        assert!(h.app.search.is_none());
        let rows = h.app.rows();
        h.app.view.reconcile(&rows, 10);
        let after: Vec<_> = h
            .app
            .view
            .selected(&rows)
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(after, ["a.txt", "c_a.txt"]);
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
        h.app.step_match(1, true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/ab.txt"))
        );
        h.app.step_match(1, true);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/a.txt")));
        h.app.step_match(-1, true);
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
        h.app.step_match(1, true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/mid.md"))
        );
        h.app.step_match(1, true);
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/zed.md"))
        );
        h.app.step_match(1, true);
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
        h.app.index = Some(super::tests::index_of(&[("a.txt", false)]));
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
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r")]);
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
        assert!(h.app.picked.is_empty(), "chevron click browses");
        assert!(h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded);
        let rows = h.app.rows();
        h.app.activate(&rows, Some((sub_idx, false)));
        assert!(h.app.picked.is_empty(), "name click browses (collapses)");
        assert!(!h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded);

        // File clicks pick.
        let rows = h.app.rows();
        let file_idx = rows.iter().position(|r| r.name == "file.txt").unwrap();
        h.app.activate(&rows, Some((file_idx, false)));
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/file.txt")]);

        // Enter picks whatever is selected — a dir included.
        h.app.picked.clear();
        let rows = h.app.rows();
        h.app.view.focus("/r/sub".into());
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/sub")]);
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
        assert!(h.app.picked.is_empty());
        assert!(h.app.tree.node_at(Path::new("/r/src")).unwrap().expanded);
        // A completed double-click on a directory browses it too — only Enter
        // confirms a directory, so exploratory clicks never pick by accident.
        h.app
            .resolve_click(&rows, 1, false, t0 + Duration::from_millis(100));
        assert!(h.app.picked.is_empty());
        // Enter on the same row picks it.
        let rows = h.app.rows();
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/src")]);
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
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/src/main.rs")]);
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
        assert!(h.app.picked.is_empty());

        // Enter on the matching file picks it.
        h.app.view.focus("/r/notes.md".into());
        h.app.activate(&rows, None);
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/notes.md")]);
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
        assert!(h.app.picked.is_empty());
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
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/src")]);
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
        h.app.rows_len = rows.len();
        h.app.scroll_rows(12, viewport);
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
        h.app.rows_len = rows.len();
        h.app.scroll_rows(20, viewport);
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
        h.app.rows_len = rows.len();
        h.app.scroll_rows(5, viewport);
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
        // without going through step_match, so a remembered match pointer goes
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
        h.app.step_match(1, true);
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
        h.app.step_match(1, true);
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
        assert!(h.app.picked.is_empty());
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
        assert!(h.app.picked.is_empty());
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
    fn a_handoff_or_a_quit_closes_the_batch() {
        // The rule the loop asks before opening a batch and after every event
        // inside one (ADR 0024, decision 4). A hand-off in the event that
        // opened the batch must stop it before a single queued event runs.
        assert_eq!(batch_step(false, false), BatchStep::Continue);
        assert_eq!(batch_step(false, true), BatchStep::Stop);
        assert_eq!(batch_step(true, false), BatchStep::Quit);
        // Quitting wins: nothing may run behind it either.
        assert_eq!(batch_step(true, true), BatchStep::Quit);
    }

    #[test]
    fn drain_batch_takes_everything_already_queued() {
        let (tx, rx) = mpsc::channel();
        for _ in 0..5 {
            tx.send(AppEvent::Shutdown).unwrap();
        }
        let mut seen = 0;
        let stop = drain_batch(&rx, Some(Instant::now() + Duration::from_secs(5)), |_| {
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
        let stop = drain_batch(&rx, Some(started + Duration::from_secs(30)), |_| {
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
        let stop = drain_batch(&rx, Some(Instant::now()), |_| {
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
            let stop = drain_batch(&rx, Some(Instant::now() + Duration::from_secs(5)), |_| {
                seen += 1;
                step
            });
            // The event that stopped the batch was handled; the rest were not.
            assert_eq!(seen, 1);
            assert_eq!(stop, expected);
        }
    }

    #[test]
    fn a_failed_detached_open_says_why_when_the_child_said_anything() {
        let reason = OpenEvent {
            program: "birch-cmux".into(),
            code: Some(1),
            reason: "no focused cmux window".into(),
        };
        assert_eq!(
            open_failure_message(&reason),
            "open failed: birch-cmux: no focused cmux window"
        );
        let silent = OpenEvent {
            program: "birch-cmux".into(),
            code: Some(3),
            reason: String::new(),
        };
        assert_eq!(
            open_failure_message(&silent),
            "open failed: birch-cmux exited with 3"
        );
        let killed = OpenEvent {
            program: "birch-cmux".into(),
            code: None,
            reason: String::new(),
        };
        assert_eq!(
            open_failure_message(&killed),
            "open failed: birch-cmux was killed"
        );
    }

    #[test]
    fn the_reason_is_the_first_real_line_cleaned_and_bounded() {
        assert_eq!(
            drain_reason(&b"\n\n  boom: it failed \nand more\n"[..]),
            "boom: it failed"
        );
        assert_eq!(drain_reason(&b"a\x07b\n"[..]), "ab");
        let long = format!("{}\n", "x".repeat(500));
        assert_eq!(drain_reason(long.as_bytes()).len(), REASON_LIMIT);
        assert_eq!(drain_reason(&b""[..]), "");
    }

    #[test]
    fn the_status_line_names_the_program_not_its_path() {
        assert_eq!(program_name("/opt/homebrew/bin/birch-cmux"), "birch-cmux");
        assert_eq!(program_name("nvim"), "nvim");
    }

    #[test]
    fn a_detached_open_that_fails_reports_it_and_one_that_works_says_nothing() {
        let mut h = harness(Mode::Tree);
        h.app.open_cmd =
            OpenCmd::from_template("/bin/sh -c 'echo boom >&2; exit 3' {}").expect("template");
        h.app.open_detached(Path::new("/r/a"));
        let failure = h
            .open_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("a failing open reports");
        assert_eq!(failure.program, "sh");
        assert_eq!(failure.code, Some(3));
        assert_eq!(failure.reason, "boom");

        h.app.open_cmd = OpenCmd::from_template("/bin/sh -c 'exit 0' {}").expect("template");
        h.app.open_detached(Path::new("/r/a"));
        assert!(
            h.open_rx.recv_timeout(Duration::from_millis(500)).is_err(),
            "a clean exit says nothing"
        );
    }

    #[test]
    fn set_open_cmd_replaces_the_command_and_leaves_absent_parts_alone() {
        let mut h = harness(Mode::Tree);
        h.app.open_cmd = OpenCmd::from_template("old {}").expect("template");
        h.app.open_cmd.set_name("host");
        h.app.open_cmd.mode = OpenMode::Detached;

        let mut req = Request::new(Verb::SetOpenCmd);
        req.value = Some("cmux open {} --window w1".into());
        assert!(h.app.ctl_response(req).0.ok);
        // Template replaced; name and mode carried over, because neither was
        // named in the request.
        assert_eq!(
            h.app.open_cmd.build(Path::new("/r/a")).first().unwrap(),
            "cmux"
        );
        assert_eq!(h.app.open_cmd.display_name(), "host");
        assert_eq!(h.app.open_cmd.mode, OpenMode::Detached);

        let mut req = Request::new(Verb::SetOpenCmd);
        req.value = Some("vim {}".into());
        req.name = Some("editor".into());
        req.detached = Some(false);
        assert!(h.app.ctl_response(req).0.ok);
        assert_eq!(h.app.open_cmd.display_name(), "editor");
        assert_eq!(h.app.open_cmd.mode, OpenMode::Terminal);
    }

    #[test]
    fn set_open_cmd_refuses_a_missing_or_unparseable_template() {
        let mut h = harness(Mode::Tree);
        let response = h.app.ctl_response(Request::new(Verb::SetOpenCmd)).0;
        assert!(!response.ok);
        assert!(response.error.unwrap_or_default().contains("template"));

        let mut req = Request::new(Verb::SetOpenCmd);
        req.value = Some("nvim +{line} {}".into());
        let response = h.app.ctl_response(req).0;
        assert!(
            !response.ok,
            "the once-documented placeholder is still refused"
        );
    }

    #[test]
    fn the_menu_row_names_the_host_when_the_command_has_a_display_name() {
        let mut h = harness(Mode::Tree);
        h.app.open_cmd = OpenCmd::from_template("birch-cmux preview {}").expect("template");
        let targets = vec![ActionTarget {
            path: PathBuf::from("/r/a"),
            is_dir: false,
            expanded: false,
            missing: false,
        }];
        let label = Action::Open.label_in(
            h.app
                .action_context(&targets, h.app.open_cmd.display_name()),
        );
        assert_eq!(label.text(), "Open with birch-cmux");
        h.app.open_cmd.set_name("cmux");
        let label = Action::Open.label_in(
            h.app
                .action_context(&targets, h.app.open_cmd.display_name()),
        );
        assert_eq!(label.text(), "Open with cmux");
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
    fn ctl_set_branch_badges_toggles_the_column() {
        let mut h = harness(Mode::Tree);
        let mut req = Request::new(Verb::Set);
        req.setting = Some(SettingKey::BranchBadges);
        assert!(h.app.settings.branch_badges, "on by default");
        req.value = Some("off".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert!(!h.app.settings.branch_badges);
        // And with it off the column is gone even where a base would resolve.
        assert!(!h.app.branch_axis());
        req.value = Some("toggle".into());
        assert!(h.app.ctl_response(req).0.ok);
        assert!(h.app.settings.branch_badges);
    }

    #[test]
    fn ctl_set_badges_takes_a_word_and_auto_returns_to_the_theme() {
        let mut h = harness(Mode::Tree);
        let mut req = Request::new(Verb::Set);
        req.setting = Some(SettingKey::Badges);
        assert_eq!(h.app.settings.badges, None, "the theme decides by default");

        req.value = Some("left".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert_eq!(h.app.settings.badges, Some(settings::BadgePlacement::Left));

        req.value = Some("none".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert_eq!(h.app.settings.badges, Some(settings::BadgePlacement::None));

        // `auto` hands the choice back rather than pinning a value.
        req.value = Some("auto".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert_eq!(h.app.settings.badges, None);

        req.value = Some("sideways".into());
        assert!(!h.app.ctl_response(req).0.ok, "not a placement");
        assert_eq!(h.app.settings.badges, None, "a refusal changes nothing");
    }

    #[test]
    fn ctl_set_scrollbar_toggles_like_its_neighbours() {
        let mut h = harness(Mode::Tree);
        let mut req = Request::new(Verb::Set);
        req.setting = Some(SettingKey::Scrollbar);
        assert!(h.app.settings.scrollbar);
        req.value = Some("off".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert!(!h.app.settings.scrollbar);
        req.value = Some("toggle".into());
        assert!(h.app.ctl_response(req.clone()).0.ok);
        assert!(h.app.settings.scrollbar);
        req.value = Some("maybe".into());
        assert!(!h.app.ctl_response(req).0.ok, "not a SettingValue");
        assert!(h.app.settings.scrollbar, "a refusal changes nothing");
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
    pub(super) fn press(app: &mut App, rows: &[Row], idx: usize, on_chevron: bool) {
        app.armed_press = Some((rows[idx].path.clone(), on_chevron));
    }

    #[test]
    fn arming_resolves_the_hit_to_a_real_path() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("a", NodeKind::File), ("b", NodeKind::File)],
        );
        let rows = h.app.rows();
        h.app.arm_press(&rows, Some((2, false)));
        assert_eq!(
            h.app.armed_press,
            Some((rows[2].path.clone(), false)),
            "the arm keys on the row's real path, not its index"
        );
        h.app.arm_press(&rows, Some((1, true)));
        assert_eq!(h.app.armed_press, Some((rows[1].path.clone(), true)));
        // A press outside the tree arms nothing.
        h.app.arm_press(&rows, None);
        assert_eq!(h.app.armed_press, None);
        // A hit past the end cannot panic or arm a phantom row.
        h.app.arm_press(&rows, Some((999, false)));
        assert_eq!(h.app.armed_press, None);
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
        super::tests::press(&mut h.app, &rows, 2, false);
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
        super::tests::press(&mut h.app, &rows, 2, false);
        h.app.resolve_release(
            &rows,
            Some((2, false)),
            input::ClickModifier::None,
            Instant::now(),
        );
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
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(
            &rows,
            Some((2, false)),
            input::ClickModifier::None,
            Instant::now(),
        );
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
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, None, input::ClickModifier::None, Instant::now());
        assert_eq!(h.app.view.selection, before);
    }

    #[test]
    fn a_release_with_no_press_does_nothing() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a", NodeKind::File)]);
        let rows = h.app.rows();
        let before = h.app.view.selection.clone();
        h.app.resolve_release(
            &rows,
            Some((1, false)),
            input::ClickModifier::None,
            Instant::now(),
        );
        assert_eq!(h.app.view.selection, before);
    }

    #[test]
    fn a_press_on_the_chevron_released_on_the_name_does_not_toggle() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("sub", NodeKind::Dir)]);
        feed(&mut h.app, "/r/sub", &[("f", NodeKind::File)]);
        let rows = h.app.rows();
        let expanded = h.app.tree.node_at(Path::new("/r/sub")).unwrap().expanded;
        super::tests::press(&mut h.app, &rows, 1, true);
        h.app.resolve_release(
            &rows,
            Some((1, false)),
            input::ClickModifier::None,
            Instant::now(),
        );
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
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, Some((1, false)), input::ClickModifier::None, t0);
        assert!(h.app.picked.is_empty(), "one click only selects");
        // A second press without its release must not activate.
        super::tests::press(&mut h.app, &rows, 1, false);
        assert!(h.app.picked.is_empty());
        h.app.resolve_release(
            &rows,
            Some((1, false)),
            input::ClickModifier::None,
            t0 + Duration::from_millis(100),
        );
        assert_eq!(
            h.app.picked.as_slice(),
            [rows[1].path.clone()],
            "two complete clicks activate"
        );
    }

    #[test]
    fn the_double_click_window_is_measured_release_to_release() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("a", NodeKind::File)]);
        let rows = h.app.rows();
        let t0 = Instant::now();
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, Some((1, false)), input::ClickModifier::None, t0);
        super::tests::press(&mut h.app, &rows, 1, false);
        // Beyond the window: a slow second click is two singles, not a double.
        h.app.resolve_release(
            &rows,
            Some((1, false)),
            input::ClickModifier::None,
            t0 + input::DOUBLE_CLICK_WINDOW + Duration::from_millis(1),
        );
        assert!(h.app.picked.is_empty());
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
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app
            .resolve_release(&rows, Some((1, false)), input::ClickModifier::None, t0);
        // A press released elsewhere is an intervening click: it resets.
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(
            &rows,
            Some((2, false)),
            input::ClickModifier::None,
            t0 + Duration::from_millis(50),
        );
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app.resolve_release(
            &rows,
            Some((1, false)),
            input::ClickModifier::None,
            t0 + Duration::from_millis(100),
        );
        assert!(
            h.app.picked.is_empty(),
            "the abandoned click reset the double"
        );
    }

    // ---- ADR 0026: the index has a lifecycle, and says which state it is in ----

    /// A partial index over the same paths — what a mid-walk chunk looks like.
    fn partial_index_of(entries: &[(&str, bool)]) -> Arc<SearchIndex> {
        Arc::new(SearchIndex::from_paths_partial(Path::new("/r"), entries))
    }

    #[test]
    fn no_matches_is_only_claimed_once_the_index_is_complete() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        h.app.search_push('z');

        // No index at all: birch cannot speak about the corpus.
        h.app.index = None;
        h.app.rematch();
        assert!(
            h.app.bottom_line().contains("indexing"),
            "{}",
            h.app.bottom_line()
        );
        assert!(!h.app.bottom_line().contains("no matches"));

        // A partial index that holds nothing matching: still not a claim.
        h.app.index = Some(partial_index_of(&[("a.txt", false)]));
        h.app.rematch();
        assert!(
            h.app.bottom_line().contains("indexing"),
            "{}",
            h.app.bottom_line()
        );
        assert!(!h.app.bottom_line().contains("no matches"));

        // Complete and holding nothing: now it is true, and said.
        h.app.index = Some(super::tests::index_of(&[("a.txt", false)]));
        h.app.rematch();
        assert!(
            h.app.bottom_line().contains("no matches"),
            "{}",
            h.app.bottom_line()
        );
    }

    #[test]
    fn a_partial_index_shows_its_matches_and_says_it_is_still_growing() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("alpha.txt", NodeKind::File)]);
        h.app.search_push('a');
        h.app.index = Some(partial_index_of(&[("alpha.txt", false)]));
        h.app.rematch();
        let line = h.app.bottom_line();
        assert!(line.contains("/1"), "counts what it has: {line}");
        assert!(
            line.contains("indexing"),
            "and says there may be more: {line}"
        );
    }

    #[test]
    fn an_arriving_chunk_leaves_a_non_matching_selection_alone() {
        // ADR 0023 lets the pane keep its tree under a search, so the user may
        // have arrowed onto a row that is not a match. A chunk arriving from
        // the index walk must not yank the cursor back to a match.
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("aaa.txt", NodeKind::File),
                ("mmm.txt", NodeKind::File),
                ("zzz.txt", NodeKind::File),
            ],
        );
        h.app.index = Some(partial_index_of(&[("aaa.txt", false)]));
        h.app.search_push('a');
        // Settle deliberately on a row the query does not match.
        h.app.view.selection = Some(PathBuf::from("/r/zzz.txt"));

        // A later chunk brings another match, sorting before the selection.
        h.app.index = Some(partial_index_of(&[("aaa.txt", false), ("mmm.txt", false)]));
        h.app.rematch_keeping_selection();
        assert_eq!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/zzz.txt")),
            "the cursor stayed where the user put it"
        );

        // Typing, by contrast, is the user asking to be taken somewhere.
        h.app.rematch();
        assert_ne!(
            h.app.view.selection.as_deref(),
            Some(Path::new("/r/zzz.txt"))
        );
    }

    #[test]
    fn an_arriving_chunk_does_not_move_a_settled_selection() {
        // The cursor must not walk under a reader's eyes as the index grows.
        let mut h = harness(Mode::Pick);
        let entries: Vec<(String, NodeKind)> = (0..6)
            .map(|i| (format!("f{i}.txt"), NodeKind::File))
            .collect();
        let refs: Vec<(&str, NodeKind)> = entries.iter().map(|(n, k)| (n.as_str(), *k)).collect();
        feed(&mut h.app, "/r", &refs);
        h.app.index = Some(partial_index_of(&[("f3.txt", false), ("f4.txt", false)]));
        h.app.search_push('f');
        // Settle on the second of the two known matches.
        h.app.step_match(1, true);
        let settled = h.app.view.selection.clone();
        assert!(settled.is_some());

        // A chunk arrives carrying matches that sort *before* the selection.
        h.app.index = Some(partial_index_of(&[
            ("f0.txt", false),
            ("f1.txt", false),
            ("f2.txt", false),
            ("f3.txt", false),
            ("f4.txt", false),
        ]));
        h.app.rematch();
        assert_eq!(
            h.app.view.selection, settled,
            "the selection stayed on the row the user had settled on"
        );
        // ...and the newly arrived rows are matches it can now step to.
        assert_eq!(h.app.search.as_ref().unwrap().matches.len(), 5);
    }

    // ---- 077: a quit during a terminal handover is not swallowed ----

    #[test]
    fn the_post_handover_drain_reports_a_quit() {
        // The drain that discards stale input after an editor exits must still
        // carry a quit out: ctl quit and SIGHUP both arrive as events, and
        // both were answered and then ignored.
        let (tx, rx) = mpsc::channel();
        tx.send(AppEvent::Input(crossterm::event::Event::FocusGained))
            .unwrap();
        tx.send(AppEvent::Shutdown).unwrap();
        tx.send(AppEvent::Input(crossterm::event::Event::FocusLost))
            .unwrap();

        let mut handled = 0;
        let stop = drain_batch(&rx, None, |pending| match pending {
            AppEvent::Input(_) => BatchStep::Continue,
            _ => {
                handled += 1;
                BatchStep::Quit
            }
        });
        assert_eq!(stop, BatchStop::Quit);
        assert_eq!(handled, 1, "the quit stopped the drain");
    }

    #[test]
    fn the_post_handover_drain_empties_the_queue() {
        // It has no time budget: every event queued while the child owned the
        // tty is stale, and leaving any behind defeats the drain.
        let (tx, rx) = mpsc::channel();
        for _ in 0..500 {
            tx.send(AppEvent::Input(crossterm::event::Event::FocusGained))
                .unwrap();
        }
        let mut seen = 0;
        let stop = drain_batch(&rx, None, |_| {
            seen += 1;
            BatchStep::Continue
        });
        assert_eq!(stop, BatchStop::Drained);
        assert_eq!(seen, 500);
    }
}

#[cfg(test)]
mod ctl_tests {
    use birch_core::NodeKind;
    use birch_core::git::parse_porcelain_v2;
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

    /// Task 074: `data` keeps carrying exactly one path — the cursor — and
    /// `paths` carries the whole selection beside it, so a client that has
    /// never heard of `paths` is unaffected (ADR 0011, additive-only).
    #[test]
    fn get_path_reports_the_cursor_in_data_and_the_selection_in_paths() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("a.txt", NodeKind::File),
                ("b.txt", NodeKind::File),
                ("c.txt", NodeKind::File),
            ],
        );
        h.app.view.focus("/r/a.txt".into());
        let response = h.app.ctl_response(request(Verb::GetPath)).0;
        assert_eq!(response.data.as_deref(), Some("a.txt"));
        assert_eq!(response.paths, Some(vec!["a.txt".into()]));

        // Extend upward, so the cursor is *not* the first selected path: `data`
        // must still be the cursor, not `paths[0]`.
        h.app.view.focus("/r/c.txt".into());
        let rows = h.app.rows();
        h.app.view.move_by_extending(&rows, -2);
        let response = h.app.ctl_response(request(Verb::GetPath)).0;
        assert_eq!(response.data.as_deref(), Some("a.txt"));
        assert_eq!(
            response.paths,
            Some(vec!["a.txt".into(), "b.txt".into(), "c.txt".into()])
        );
    }

    /// The picker without `--multi` is exactly what it always was: `Shift` with
    /// a movement key is just that movement, and Enter confirms one path.
    #[test]
    fn the_picker_is_single_select_without_multi() {
        for (multi, expected) in [(false, 1), (true, 3)] {
            let mut h = harness(Mode::Pick);
            h.app.multi = multi;
            feed(
                &mut h.app,
                "/r",
                &[
                    ("a.txt", NodeKind::File),
                    ("b.txt", NodeKind::File),
                    ("c.txt", NodeKind::File),
                ],
            );
            let rows = h.app.rows();
            h.app.view.focus("/r/a.txt".into());
            if multi {
                h.app.view.move_by_extending(&rows, 2);
            } else {
                h.app.view.move_by(&rows, 2);
            }
            h.app.activate(&rows, None);
            assert_eq!(h.app.picked.len(), expected, "multi = {multi}");
        }
    }

    // ---- 071: the menu targets the selection ----

    /// Right-clicking inside the selection leaves it alone and acts on all of
    /// it; outside it, the selection collapses to that row first (ADR 0029).
    #[test]
    fn a_right_click_only_disturbs_the_selection_when_it_must() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("a.txt", NodeKind::File),
                ("b.txt", NodeKind::File),
                ("c.txt", NodeKind::File),
            ],
        );
        let rows = h.app.rows();
        h.app.view.focus("/r/a.txt".into());
        h.app.view.move_by_extending(&rows, 1); // a.txt + b.txt

        // Inside the selection: untouched, and the menu sees both rows.
        h.app.view.on_single_click_if_unselected(&rows, 1);
        assert_eq!(h.app.action_targets(&rows).len(), 2);

        // Outside it: the selection collapses to the clicked row.
        h.app.view.on_single_click_if_unselected(&rows, 3);
        let targets = h.app.action_targets(&rows);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].path, PathBuf::from("/r/c.txt"));
    }

    /// The projection is what the menu contains: a file offers Open alone, a
    /// directory offers Reveal Root Here as well, and a plural selection drops
    /// the single-target action.
    #[test]
    fn the_menu_is_generated_from_what_is_available() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("a.txt", NodeKind::File)],
        );
        let rows = h.app.rows();

        h.app.view.focus("/r/src".into());
        let targets = h.app.action_targets(&rows);
        let context = ActionContext {
            targets: &targets,
            picker: false,
            open_is_platform: false,
            open_program: "nvim",
            system_program: "open",
            undo: None,
            pending: None,
            cursor: targets.first(),
        };
        assert!(Action::Open.available(context));
        assert!(Action::RevealRootHere.available(context));

        h.app.view.focus("/r/a.txt".into());
        let targets = h.app.action_targets(&rows);
        let context = ActionContext {
            targets: &targets,
            picker: false,
            open_is_platform: false,
            open_program: "nvim",
            system_program: "open",
            undo: None,
            pending: None,
            cursor: targets.first(),
        };
        assert!(Action::Open.available(context));
        assert!(
            !Action::RevealRootHere.available(context),
            "a file cannot become a root"
        );

        h.app.view.focus("/r/src".into());
        h.app.view.move_by_extending(&rows, 1);
        let targets = h.app.action_targets(&rows);
        let context = ActionContext {
            targets: &targets,
            picker: false,
            open_is_platform: false,
            open_program: "nvim",
            system_program: "open",
            undo: None,
            pending: None,
            cursor: targets.first(),
        };
        assert!(
            !Action::RevealRootHere.available(context),
            "re-rooting is single-target"
        );
    }

    /// The pane a synthetic right-click lands in. 40x20 is roomy enough that
    /// the scrollbar only appears when there are more rows than fit.
    fn pane() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 20,
        }
    }

    /// `hit_test` says `None` for the empty space below the last row, for the
    /// scrollbar column, and for the status line alike. Only the first targets
    /// the root; treating them alike destroyed a multi-selection when the
    /// scrollbar was right-clicked.
    #[test]
    fn a_right_click_on_furniture_targets_nothing() {
        let mut h = harness(Mode::Tree);
        let names: Vec<_> = (0..40).map(|i| format!("f{i:02}.txt")).collect();
        let entries: Vec<_> = names.iter().map(|n| (n.as_str(), NodeKind::File)).collect();
        feed(&mut h.app, "/r", &entries);
        let rows = h.app.rows();
        let area = pane();

        // A row: targeted by path, not by cell.
        assert_eq!(
            h.app.right_target(area, &rows, 4, 2),
            Some(Some(rows[2].path.clone()))
        );
        // The same row, a column over: still the same target, so a pointer that
        // drifts between press and release still opens the menu.
        assert_eq!(
            h.app.right_target(area, &rows, 5, 2),
            h.app.right_target(area, &rows, 4, 2)
        );
        // The scrollbar column is inert for the right button as for the left.
        assert_eq!(h.app.right_target(area, &rows, area.width - 1, 2), None);
        // The status line is not the tree.
        assert_eq!(h.app.right_target(area, &rows, 4, area.height - 1), None);
        // A pane too small to draw a menu never opens one.
        let tiny = Rect {
            width: 2,
            height: 2,
            ..area
        };
        assert_eq!(h.app.right_target(tiny, &rows, 0, 0), None);
    }

    /// Below the last row targets the root — which is what makes the root menu
    /// reachable with nothing useful selected.
    #[test]
    fn a_right_click_below_the_rows_targets_the_root() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        assert_eq!(h.app.right_target(pane(), &rows, 4, 8), Some(None));

        h.app.view.focus("/r/a.txt".into());
        assert!(h.app.open_menu(&rows, None, 4, 8));
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r")));
    }

    /// A row tracked in git but gone from disk has no available action, so no
    /// menu opens — and the selection must not be destroyed on the way to
    /// finding that out.
    #[test]
    fn a_right_click_on_a_missing_row_leaves_the_selection_alone() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        // Tracked in git, gone from disk: the row renders so the deletion is
        // visible, but nothing can act on it.
        h.app.git_state = Some(Arc::new(parse_porcelain_v2(
            b"1 .D N... 100644 100644 000000 a a gone.rs\0",
            Path::new("/r"),
        )));
        let rows = h.app.rows();
        let missing = rows
            .iter()
            .position(|row| row.name == "gone.rs")
            .expect("the deleted row renders");
        assert!(rows[missing].missing);
        h.app.view.focus("/r/a.txt".into());
        assert_eq!(h.app.right_target(pane(), &rows, 4, missing as u16), None);
        assert_eq!(h.app.view.selection.as_deref(), Some(Path::new("/r/a.txt")));
    }

    /// Picking from the menu must end the picker where it happened, not on
    /// whatever key arrives next.
    #[test]
    fn open_from_the_menu_confirms_the_pick_immediately() {
        let mut h = harness(Mode::Pick);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus("/r/a.txt".into());
        assert!(
            h.app
                .open_menu(&rows, Some(PathBuf::from("/r/a.txt")), 4, 1)
        );

        assert_eq!(
            h.app.perform(Action::Open),
            Performed::Nothing,
            "the picker confirms"
        );
        assert_eq!(h.app.picked.as_slice(), [PathBuf::from("/r/a.txt")]);
        assert!(
            h.app.finished(false),
            "the picker exits on the pick, not one event later"
        );
    }

    #[test]
    fn open_from_the_menu_toggles_a_directory_and_reports_a_file() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("a.txt", NodeKind::File)],
        );
        h.app.view.focus("/r/src".into());
        assert_eq!(h.app.perform(Action::Open), Performed::Nothing);
        assert!(
            h.app.tree.node_at(Path::new("/r/src")).unwrap().expanded,
            "Open toggles a directory, exactly as Enter does"
        );

        // On a file it hands the path back for the caller to open, rather than
        // reaching for the terminal itself.
        h.app.view.focus("/r/a.txt".into());
        assert_eq!(
            h.app.perform(Action::Open),
            Performed::Open(PathBuf::from("/r/a.txt"))
        );
        assert!(
            !h.app.finished(false),
            "the tree does not exit on an action"
        );
    }

    #[test]
    fn reveal_root_here_re_roots_the_tree() {
        let tmp = std::env::temp_dir().join(format!("birch-menu-root-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        let real = tmp.canonicalize().unwrap();

        let mut h = harness(Mode::Tree);
        h.app.root = real.clone();
        h.app.tree = Tree::new(real.clone());
        h.app.tree.set_expanded(&real, true);
        h.app.tree.apply(TreeDelta::Snapshot {
            dir: real.clone(),
            entries: vec![birch_core::Entry {
                name: "src".into(),
                kind: NodeKind::Dir,
            }],
        });
        h.app.view.focus(real.join("src"));

        assert_eq!(h.app.perform(Action::RevealRootHere), Performed::Nothing);
        assert_eq!(h.app.root, real.join("src"));
        assert!(h.app.status.is_empty(), "no error was reported");

        std::fs::remove_dir_all(&tmp).unwrap();
    }

    /// Every branch of the menu's input handling, which two of this sprint's
    /// bugs lived in.
    #[test]
    fn the_menu_handles_its_own_input() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        let rows = h.app.rows();
        let area = pane();
        let open = |h: &mut App| {
            h.view.focus("/r/src".into());
            let rows = h.rows();
            assert!(h.open_menu(&rows, Some(PathBuf::from("/r/src")), 2, 2));
        };

        // Arrows step entries and skip the rule between the groups; Home/End
        // reach the ends.
        open(&mut h.app);
        // Derived from the menu itself: the projection grows as actions land,
        // and this test is about traversal rather than about which rows exist.
        let entries: Vec<Action> = menu_ref(&h.app.menu)
            .rows()
            .iter()
            .filter_map(|row| match row {
                birch_core::action::MenuRow::Entry(action) => Some(*action),
                birch_core::action::MenuRow::Separator => None,
            })
            .collect();
        assert!(entries.len() >= 3, "several groups to cross");

        // Stepping crosses the rules without ever landing on one.
        assert_eq!(h.app.menu_input(area, InputAction::Down), MenuStep::Keep);
        assert_eq!(menu_ref(&h.app.menu).selected_action(), Some(entries[1]));
        h.app.menu_input(area, InputAction::Up);
        assert_eq!(menu_ref(&h.app.menu).selected_action(), Some(entries[0]));
        h.app.menu_input(area, InputAction::End);
        assert_eq!(
            menu_ref(&h.app.menu).selected_action(),
            entries.last().copied()
        );
        h.app.menu_input(area, InputAction::Home);
        assert_eq!(menu_ref(&h.app.menu).selected_action(), Some(entries[0]));

        // Enter invokes what is highlighted; Esc closes and changes nothing.
        assert_eq!(
            h.app.menu_input(area, InputAction::Enter),
            MenuStep::Invoke(Action::Open)
        );
        assert_eq!(h.app.menu_input(area, InputAction::Esc), MenuStep::Close);

        // A printable closes the menu and reaches the search — ADR 0008 has no
        // exception for a menu.
        assert_eq!(
            h.app.menu_input(area, InputAction::Char('x')),
            MenuStep::Search('x')
        );

        let layout = h
            .app
            .menu
            .as_mut()
            .unwrap()
            .layout(area.x, area.y, area.width, area.height);

        // Hover moves the highlight; a press inside keeps the menu, a press
        // outside dismisses it.
        // Located by kind rather than by index: the projection grows as
        // actions land, and this test is about the mechanism.
        let rule = menu_ref(&h.app.menu)
            .rows()
            .iter()
            .position(|row| matches!(row, birch_core::action::MenuRow::Separator))
            .expect("the menu has groups");
        let after_rule = rule + 1;
        let hovered = menu_ref(&h.app.menu)
            .action_at(after_rule)
            .expect("the next group's first entry");
        h.app.menu_input(
            area,
            InputAction::Hover {
                column: layout.x,
                row: layout.y + 1 + after_rule as u16,
            },
        );
        assert_eq!(menu_ref(&h.app.menu).selected_action(), Some(hovered));
        assert_eq!(
            h.app.menu_input(
                area,
                InputAction::Press {
                    column: layout.x,
                    row: layout.y + 1
                }
            ),
            MenuStep::Keep
        );
        assert_eq!(
            h.app.menu_input(
                area,
                InputAction::Press {
                    column: layout.x + layout.width,
                    row: layout.y
                }
            ),
            MenuStep::Close
        );

        // A click on the rule between two groups does nothing. Before this was
        // fixed it invoked whatever entry the highlight was standing on.
        h.app.menu_input(area, InputAction::Home);
        let separator = InputAction::Release {
            column: layout.x,
            row: layout.y + 1 + rule as u16,
            modifier: input::ClickModifier::None,
        };
        assert_eq!(h.app.menu_input(area, separator), MenuStep::Keep);
        // ...while a click on an entry invokes that entry, not the highlight.
        let entry = InputAction::Release {
            column: layout.x,
            row: layout.y + 1 + after_rule as u16,
            modifier: input::ClickModifier::None,
        };
        assert_eq!(h.app.menu_input(area, entry), MenuStep::Invoke(hovered));

        // A right-click elsewhere moves the menu rather than stranding it.
        // "Elsewhere" is computed rather than guessed: the menu grows as
        // actions land, and a fixed column drifts inside it.
        open(&mut h.app);
        let layout = menu_mut(&mut h.app.menu).layout(area.x, area.y, area.width, area.height);
        let outside = if layout.x > 0 {
            layout.x - 1
        } else {
            layout.x + layout.width
        };
        assert!(
            !menu_ref(&h.app.menu).covers(layout, outside, 1),
            "the point is outside the menu"
        );
        let elsewhere = InputAction::RightPress {
            column: outside,
            row: 1,
        };
        assert_eq!(
            h.app.menu_input(area, elsewhere),
            MenuStep::Reopen(Some(Some(rows[1].path.clone())))
        );
    }

    /// A copy resolves through the same `format_path` the socket uses, so the
    /// menu and `birch ctl get-path` cannot disagree about what a form means.
    #[test]
    fn copying_joins_the_selection_in_tree_order() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[("src", NodeKind::Dir), ("a.txt", NodeKind::File)],
        );
        drain_expands(&h.source_rx);
        feed(&mut h.app, "/r/src", &[("main.rs", NodeKind::File)]);
        h.app.tree.set_expanded(Path::new("/r/src"), true);
        let rows = h.app.rows();
        h.app.view.focus("/r/src/main.rs".into());
        h.app.view.move_by_extending(&rows, 1);

        let targets = h.app.action_targets(&rows);
        assert_eq!(targets.len(), 2);
        let text = |form| {
            targets
                .iter()
                .map(|t| h.app.format_path(&t.path, form))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(text(PathForm::Name), "main.rs\na.txt");
        assert_eq!(text(PathForm::Rel), "src/main.rs\na.txt");
        assert_eq!(text(PathForm::Abs), "/r/src/main.rs\n/r/a.txt");
    }

    /// The status line says what was copied, and pluralises on the count rather
    /// than claiming the clipboard now holds it — with OSC 52 nothing can know.
    #[test]
    fn the_copy_message_names_the_form_and_the_count() {
        assert_eq!(copied_message(1, CopyForm::Name), "Copied name");
        assert_eq!(
            copied_message(1, CopyForm::Relative),
            "Copied relative path"
        );
        assert_eq!(
            copied_message(3, CopyForm::Absolute),
            "Copied 3 absolute paths"
        );
    }

    /// OSC 52 comes back for the caller to write; a native tool leaves nothing
    /// to do. Either way the status line reports it.
    #[test]
    fn a_copy_reports_what_it_did() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus("/r/a.txt".into());
        let targets = h.app.action_targets(&rows);

        // Drive the transport directly: choosing it reads the real environment,
        // which a test must not depend on.
        let sequence = clipboard::copy("a.txt", clipboard::Transport::Osc52).unwrap();
        assert_eq!(sequence, Some(clipboard::osc52("a.txt")));

        let performed = h.app.copy_selection(&targets, CopyForm::Name);
        assert!(matches!(
            performed,
            Performed::Write(_) | Performed::Nothing
        ));
        assert_eq!(h.app.status, "Copied name");
    }

    /// The keyboard route in has no pointer to reconcile with the selection, so
    /// unlike a right-click outside it, it disturbs nothing.
    #[test]
    fn opening_the_menu_from_the_keyboard_leaves_the_selection_alone() {
        let mut h = harness(Mode::Tree);
        feed(
            &mut h.app,
            "/r",
            &[
                ("a.txt", NodeKind::File),
                ("b.txt", NodeKind::File),
                ("c.txt", NodeKind::File),
            ],
        );
        let rows = h.app.rows();
        h.app.view.focus("/r/a.txt".into());
        h.app.view.move_by_extending(&rows, 1);
        assert_eq!(h.app.view.selection_len(), 2);

        let point = h
            .app
            .cursor_point(pane(), &rows)
            .expect("the cursor is on screen");
        assert!(
            h.app
                .open_menu(&rows, h.app.view.selection.clone(), point.0, point.1)
        );
        assert_eq!(
            h.app.view.selection_len(),
            2,
            "the selection survives the menu opening"
        );
        assert_eq!(h.app.action_targets(&rows).len(), 2);
    }

    /// The menu lands on the cursor's own row, indented to where its text
    /// starts rather than at column zero.
    #[test]
    fn the_keyboard_menu_opens_at_the_cursor_row() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("src", NodeKind::Dir)]);
        drain_expands(&h.source_rx);
        feed(&mut h.app, "/r/src", &[("main.rs", NodeKind::File)]);
        h.app.tree.set_expanded(Path::new("/r/src"), true);
        let rows = h.app.rows();

        // Rows: /r (depth 0), src (1), main.rs (2).
        h.app.view.focus("/r".into());
        assert_eq!(h.app.cursor_point(pane(), &rows), Some((0, 0)));
        h.app.view.focus("/r/src/main.rs".into());
        let (x, y) = h.app.cursor_point(pane(), &rows).expect("on screen");
        assert_eq!(y, 2, "the cursor's own row");
        assert_eq!(x, 2 * render::INDENT_WIDTH, "indented to the row's text");

        // Scrolled out of the viewport, there is nowhere to put it.
        h.app.view.scroll = 99;
        assert_eq!(h.app.cursor_point(pane(), &rows), None);
    }

    /// A scratch tree on the real filesystem, since operations write to it.
    fn scratch(tag: &str) -> PathBuf {
        // No test reaches the developer's real trash. Installed here rather
        // than per-test because every test that trashes builds a scratch
        // directory first, so future ones inherit it.
        birch_core::ops::redirect_trash_for_tests(
            std::env::temp_dir().join(format!("birch-test-trash-{}", std::process::id())),
        );
        let path = std::env::temp_dir().join(format!(
            "birch-app-{}-{tag}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path.canonicalize().unwrap()
    }

    fn rooted(root: &Path, entries: &[(&str, NodeKind)]) -> super::tests::Harness {
        let mut h = harness(Mode::Tree);
        h.app.root = root.to_path_buf();
        h.app.tree = Tree::new(root.to_path_buf());
        h.app.tree.set_expanded(root, true);
        h.app.tree.apply(TreeDelta::Snapshot {
            dir: root.to_path_buf(),
            entries: entries
                .iter()
                .map(|(name, kind)| birch_core::Entry {
                    name: (*name).into(),
                    kind: *kind,
                })
                .collect(),
        });
        h
    }

    /// Typing goes to the field rather than the search, `Enter` commits, and
    /// nothing touches the tree until it does (ADR 0030).
    #[test]
    fn an_inline_rename_only_touches_disk_on_enter() {
        let root = scratch("rename");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus(root.join("a.txt"));

        h.app.begin_edit(&rows, Action::Rename);
        assert!(h.app.edit.is_some(), "the field is open");
        // A printable reaches the field, not the search.
        h.app.handle_edit_input(InputAction::Char('b'));
        assert!(h.app.search.is_none(), "typing did not start a search");
        assert!(root.join("a.txt").exists(), "nothing has happened yet");

        h.app.handle_edit_input(InputAction::Enter);
        assert!(h.app.edit.is_none());
        assert!(root.join("ab.txt").exists() && !root.join("a.txt").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_abandoned_edit_leaves_nothing_behind() {
        let root = scratch("abandon");
        let mut h = rooted(&root, &[]);
        let rows = h.app.rows();
        h.app.view.focus(root.clone());

        h.app.begin_edit(&rows, Action::NewFile);
        for c in "ghost.txt".chars() {
            h.app.handle_edit_input(InputAction::Char(c));
        }
        h.app.handle_edit_input(InputAction::Esc);
        assert!(h.app.edit.is_none());
        assert!(!root.join("ghost.txt").exists(), "Esc wrote nothing");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_refused_name_keeps_the_field_and_the_text() {
        // Closing the field on a clash threw away what was typed and left the
        // tree unchanged behind it, which reads as a create that silently
        // overwrote the file it collided with.
        let root = scratch("clash");
        std::fs::write(root.join("taken.txt"), b"KEEP").unwrap();
        let mut h = rooted(&root, &[("taken.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus(root.clone());

        h.app.begin_edit(&rows, Action::NewFile);
        for c in "taken.txt".chars() {
            h.app.handle_edit_input(InputAction::Char(c));
        }
        h.app.handle_edit_input(InputAction::Enter);

        assert!(h.app.edit.is_some(), "the field stayed open");
        assert_eq!(
            h.app.edit.as_ref().map(|e| e.buffer()),
            Some("taken.txt"),
            "with the text intact, so a typo costs a keystroke and not a retype"
        );
        assert!(
            h.app.status.contains("already exists"),
            "and it says why: {:?}",
            h.app.status
        );
        assert_eq!(
            std::fs::read(root.join("taken.txt")).unwrap(),
            b"KEEP",
            "the file it collided with is untouched"
        );

        // Correcting the name in the same field commits.
        for _ in 0..9 {
            h.app.handle_edit_input(InputAction::Backspace);
        }
        for c in "free.txt".chars() {
            h.app.handle_edit_input(InputAction::Char(c));
        }
        h.app.handle_edit_input(InputAction::Enter);
        assert!(h.app.edit.is_none(), "the corrected name committed");
        assert!(root.join("free.txt").exists());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn cut_and_paste_moves_and_copy_and_paste_duplicates() {
        let root = scratch("clipboard");
        std::fs::write(root.join("a.txt"), b"DATA").unwrap();
        std::fs::create_dir(root.join("dest")).unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File), ("dest", NodeKind::Dir)]);
        let rows = h.app.rows();

        // Nothing held: paste is absent from the menu rather than inert.
        h.app.view.focus(root.join("dest"));
        let targets = h.app.action_targets(&rows);
        assert!(!Action::Paste.available(h.app.action_context(&targets, "vim")));

        h.app.view.focus(root.join("a.txt"));
        h.app.hold(&rows, false);
        assert_eq!(h.app.pending_label.as_deref(), Some("a.txt"));

        h.app.view.focus(root.join("dest"));
        let rows = h.app.rows();
        h.app.paste(&rows);
        assert!(
            root.join("dest/a.txt").exists() && !root.join("a.txt").exists(),
            "a cut paste moves"
        );
        assert!(h.app.pending.is_none(), "the clipboard is spent");

        // ...and it is one undo step.
        h.app.undo();
        assert!(root.join("a.txt").exists() && !root.join("dest/a.txt").exists());

        // A copy paste duplicates, leaving the original where it was.
        let rows = h.app.rows();
        h.app.view.focus(root.join("a.txt"));
        h.app.hold(&rows, true);
        h.app.view.focus(root.join("dest"));
        let rows = h.app.rows();
        h.app.paste(&rows);
        assert_eq!(std::fs::read(root.join("dest/a.txt")).unwrap(), b"DATA");
        assert!(root.join("a.txt").exists(), "the original stayed");

        // Undoing a paste-as-copy trashes what it made, never the original.
        h.app.undo();
        assert!(!root.join("dest/a.txt").exists());
        assert!(root.join("a.txt").exists(), "the original is untouched");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_paste_where_it_already_is_says_so_rather_than_claiming_a_move() {
        let root = scratch("clipboard-noop");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();

        h.app.view.focus(root.join("a.txt"));
        h.app.hold(&rows, false);
        // The root is a.txt's own directory.
        h.app.view.focus(root.clone());
        let rows = h.app.rows();
        h.app.paste(&rows);
        assert!(
            h.app.status.contains("already in"),
            "an operation that did nothing says nothing happened: {:?}",
            h.app.status
        );
        assert!(h.app.undo_label.is_none(), "and records no undo step");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn undo_takes_back_a_rename_and_then_a_create() {
        let root = scratch("undo");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus(root.join("a.txt"));

        // Nothing done yet: the action is absent rather than inert (ADR 0029).
        let targets = h.app.action_targets(&rows);
        assert!(
            !Action::Undo.available(h.app.action_context(&targets, "vim")),
            "there is nothing to take back"
        );

        // The field opens pre-filled with the current name, cursor at the
        // extension boundary, so one character makes `ab.txt`.
        h.app.begin_edit(&rows, Action::Rename);
        h.app.handle_edit_input(InputAction::Char('b'));
        h.app.handle_edit_input(InputAction::Enter);
        assert!(root.join("ab.txt").exists());
        assert_eq!(h.app.undo_label.as_deref(), Some("Rename of ab.txt"));

        h.app.undo();
        assert!(
            root.join("a.txt").exists() && !root.join("ab.txt").exists(),
            "the name came back"
        );
        assert_eq!(h.app.undo_label, None, "and the stack is empty again");

        // Undoing a create trashes what it made rather than unlinking it: the
        // file may have been written to since birch made it.
        let rows = h.app.rows();
        h.app.view.focus(root.clone());
        h.app.begin_edit(&rows, Action::NewFile);
        for c in "made.txt".chars() {
            h.app.handle_edit_input(InputAction::Char(c));
        }
        h.app.handle_edit_input(InputAction::Enter);
        assert!(root.join("made.txt").exists());
        assert_eq!(h.app.undo_label.as_deref(), Some("New made.txt"));

        h.app.undo();
        assert!(!root.join("made.txt").exists(), "it went to the trash");
        assert!(
            h.app.status.contains("trash"),
            "and says so: {}",
            h.app.status
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn undo_after_a_delete_brings_the_file_back() {
        // And does not reverse the rename underneath it, which would be the
        // destructive surprise ADR 0033 exists to prevent.
        let root = scratch("undo-delete");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        std::fs::write(root.join("gone.txt"), b"y").unwrap();
        let mut h = rooted(
            &root,
            &[("a.txt", NodeKind::File), ("gone.txt", NodeKind::File)],
        );
        let rows = h.app.rows();

        h.app.view.focus(root.join("a.txt"));
        h.app.begin_edit(&rows, Action::Rename);
        h.app.handle_edit_input(InputAction::Char('b'));
        h.app.handle_edit_input(InputAction::Enter);

        h.app.trash_now(vec![root.join("gone.txt")]);
        assert_eq!(h.app.undo_label.as_deref(), Some("Delete of gone.txt"));

        h.app.undo();
        assert!(
            root.join("gone.txt").exists(),
            "the file came back out of the trash: {}",
            h.app.status
        );
        assert!(
            h.app.status.starts_with("Restored"),
            "and says so: {}",
            h.app.status
        );
        assert!(
            root.join("ab.txt").exists(),
            "the rename underneath was not touched"
        );
        assert_eq!(
            h.app.undo_label.as_deref(),
            Some("Rename of ab.txt"),
            "the rename is still there for the next Ctrl-Z"
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_empty_name_keeps_the_field_open_too() {
        let root = scratch("empty-name");
        let mut h = rooted(&root, &[]);
        let rows = h.app.rows();
        h.app.view.focus(root.clone());

        h.app.begin_edit(&rows, Action::NewFile);
        h.app.handle_edit_input(InputAction::Enter);
        assert!(h.app.edit.is_some(), "nothing to commit, nothing to close");
        assert!(h.app.status.contains("required"));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn creating_makes_intermediates_and_a_trailing_slash_makes_a_directory() {
        let root = scratch("create");
        let mut h = rooted(&root, &[]);
        let rows = h.app.rows();
        h.app.view.focus(root.clone());

        for (input, check) in [("deep/deeper/c.txt", false), ("sub/", true)] {
            h.app.begin_edit(&rows, Action::NewFile);
            for c in input.chars() {
                h.app.handle_edit_input(InputAction::Char(c));
            }
            h.app.handle_edit_input(InputAction::Enter);
            let made = root.join(input.trim_end_matches('/'));
            assert_eq!(made.is_dir(), check, "{input}");
            assert!(made.exists(), "{input}");
        }

        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A delete never asks. It goes to the trash and `Ctrl-Z` brings it back;
    /// a prompt guarding a reversible act is friction (ADR 0031).
    #[test]
    fn a_delete_never_asks_whatever_git_knows() {
        let root = scratch("delete");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus(root.join("a.txt"));

        // Outside a repository, where git has no copy of anything — the case
        // that used to prompt.
        h.app.begin_delete(&rows);
        assert!(
            !root.join("a.txt").exists(),
            "it went straight to the trash"
        );
        assert!(h.app.status.contains("trash"), "{}", h.app.status);

        // ...and it is undoable, which is what makes the prompt unnecessary.
        h.app.undo();
        assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"x");

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_delete_moves_the_path_to_the_trash() {
        let root = scratch("trash");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        h.app.view.focus(root.join("a.txt"));

        h.app.begin_delete(&rows);
        assert!(!root.join("a.txt").exists(), "it left the tree");
        assert!(
            h.app.status.contains("trash"),
            "the status says where it went: {:?}",
            h.app.status
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_set_of_paths_is_named_by_count() {
        assert_eq!(describe(&[PathBuf::from("/r/a.txt")]), "a.txt");
        assert_eq!(
            describe(&[PathBuf::from("/r/a"), PathBuf::from("/r/b")]),
            "2 items"
        );
    }

    /// A press that stays on its row is still a click; one that leaves it
    /// becomes a drag. No threshold to tune (ADR 0032).
    #[test]
    fn a_drag_begins_only_when_the_pointer_leaves_its_row() {
        let root = scratch("drag-begin");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        let mut h = rooted(&root, &[("sub", NodeKind::Dir), ("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        // Rows: root (0), sub (1), a.txt (2).
        super::tests::press(&mut h.app, &rows, 2, false);
        h.app.update_drag(pane(), &rows, 4, 2);
        assert!(h.app.drag.is_none(), "still on its own row: a click");

        h.app.update_drag(pane(), &rows, 4, 1);
        let drag = h.app.drag.as_ref().expect("it left the row");
        assert_eq!(drag.paths, vec![root.join("a.txt")]);
        assert_eq!(
            drag.target.as_ref().map(|t| t.directory.clone()),
            Some(root.join("sub"))
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_drop_moves_the_paths_and_says_where_they_went() {
        let root = scratch("drag-drop");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        let mut h = rooted(&root, &[("sub", NodeKind::Dir), ("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        super::tests::press(&mut h.app, &rows, 2, false);
        h.app.update_drag(pane(), &rows, 4, 1);
        h.app.finish_drag();

        assert!(root.join("sub/a.txt").is_file() && !root.join("a.txt").exists());
        assert!(h.app.status.contains("sub"), "{:?}", h.app.status);
        assert!(h.app.drag.is_none());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Every refusal is visible during the drag rather than discovered after
    /// it, and a refused drop moves nothing.
    #[test]
    fn a_refused_target_is_refused_before_release() {
        let root = scratch("drag-refuse");
        std::fs::create_dir_all(root.join("outer/inner")).unwrap();
        std::fs::write(root.join("outer/inner/a.txt"), b"x").unwrap();
        // `keep.txt` beside `inner` stops the two directories compacting into
        // one chain row, which would otherwise make `outer` unaddressable.
        std::fs::write(root.join("outer/keep.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("outer", NodeKind::Dir)]);
        h.app.tree.set_expanded(&root.join("outer"), true);
        h.app.tree.apply(TreeDelta::Snapshot {
            dir: root.join("outer"),
            entries: vec![
                birch_core::Entry {
                    name: "inner".into(),
                    kind: NodeKind::Dir,
                },
                birch_core::Entry {
                    name: "keep.txt".into(),
                    kind: NodeKind::File,
                },
            ],
        });
        let rows = h.app.rows();
        assert_eq!(rows.len(), 4, "root, outer, inner, keep.txt");
        // Rows: root (0), outer (1), inner (2).
        super::tests::press(&mut h.app, &rows, 1, false);
        h.app.update_drag(pane(), &rows, 4, 2);
        let target = h.app.drag.as_ref().unwrap().target.as_ref().unwrap();
        assert!(
            target.refusal.is_some(),
            "a folder cannot move inside itself"
        );
        assert!(h.app.status.contains("inside itself"), "{:?}", h.app.status);

        h.app.finish_drag();
        assert!(root.join("outer/inner").is_dir(), "nothing moved");
        assert!(h.app.status.starts_with("Not moved"), "{:?}", h.app.status);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_drop_onto_the_current_parent_is_refused_as_already_there() {
        let root = scratch("drag-noop");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        let rows = h.app.rows();
        super::tests::press(&mut h.app, &rows, 1, false);
        // Row 0 is the root, which is already a.txt's parent.
        h.app.update_drag(pane(), &rows, 4, 0);
        let target = h.app.drag.as_ref().unwrap().target.as_ref().unwrap();
        assert_eq!(target.refusal.as_deref(), Some("already there"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Dragging moves files, so it is a mutation — and mutations never happen
    /// in the picker, which the gesture must state itself because it does not
    /// go through `Action::available` (ADR 0032).
    #[test]
    fn a_drag_does_nothing_in_the_picker() {
        let root = scratch("pick-drag");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        std::fs::create_dir(root.join("sub")).unwrap();
        let mut h = rooted(&root, &[("sub", NodeKind::Dir), ("a.txt", NodeKind::File)]);
        h.app.mode = Mode::Pick;
        let rows = h.app.rows();

        super::tests::press(&mut h.app, &rows, 2, false);
        h.app.update_drag(pane(), &rows, 4, 1);
        assert!(h.app.drag.is_none(), "no drag begins in the picker");
        h.app.finish_drag();
        assert!(root.join("a.txt").is_file(), "and nothing moved");
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A compacted chain renders as `a/b/c` while its path is the tail.
    /// Pre-filling the label would resolve against the tail's parent and move
    /// the directory somewhere nobody asked for.
    #[test]
    fn renaming_a_compacted_chain_edits_its_tail_name() {
        let root = scratch("chain-rename");
        std::fs::create_dir_all(root.join("src/utils")).unwrap();
        let mut h = rooted(&root, &[("src", NodeKind::Dir)]);
        h.app.tree.set_expanded(&root.join("src"), true);
        h.app.tree.apply(TreeDelta::Snapshot {
            dir: root.join("src"),
            entries: vec![birch_core::Entry {
                name: "utils".into(),
                kind: NodeKind::Dir,
            }],
        });
        let rows = h.app.rows();
        let chain = rows
            .iter()
            .find(|row| row.name.contains('/'))
            .expect("a chain formed");
        assert_eq!(chain.path, root.join("src/utils"));

        h.app.view.focus(chain.path.clone());
        h.app.begin_edit(&rows, Action::Rename);
        assert_eq!(
            h.app.edit.as_ref().unwrap().buffer(),
            "utils",
            "the tail's name, not the chain's label"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// Two dragged rows sharing a basename collide with each other, which the
    /// filesystem cannot yet show.
    #[test]
    fn a_drag_refuses_a_collision_inside_its_own_set() {
        let root = scratch("drag-set-collide");
        std::fs::create_dir_all(root.join("x")).unwrap();
        std::fs::create_dir_all(root.join("y")).unwrap();
        std::fs::create_dir_all(root.join("z")).unwrap();
        std::fs::write(root.join("x/a.txt"), b"x").unwrap();
        std::fs::write(root.join("y/a.txt"), b"x").unwrap();
        let mut h = rooted(
            &root,
            &[
                ("x", NodeKind::Dir),
                ("y", NodeKind::Dir),
                ("z", NodeKind::Dir),
            ],
        );
        for dir in ["x", "y"] {
            h.app.tree.set_expanded(&root.join(dir), true);
            h.app.tree.apply(TreeDelta::Snapshot {
                dir: root.join(dir),
                entries: vec![birch_core::Entry {
                    name: "a.txt".into(),
                    kind: NodeKind::File,
                }],
            });
        }
        h.app.tree.apply(TreeDelta::Snapshot {
            dir: root.join("z"),
            entries: Vec::new(),
        });
        let rows = h.app.rows();
        let find = |name: &str, parent: &str| {
            rows.iter()
                .position(|r| r.path == root.join(parent).join(name))
                .expect("row")
        };
        let (xa, ya) = (find("a.txt", "x"), find("a.txt", "y"));
        h.app.view.focus(root.join("x/a.txt"));
        h.app.view.toggle_selected(&rows, ya);
        assert_eq!(h.app.view.selection_len(), 2);

        super::tests::press(&mut h.app, &rows, xa, false);
        let z = rows.iter().position(|r| r.path == root.join("z")).unwrap();
        h.app.update_drag(pane(), &rows, 4, z as u16);
        let drag = h.app.drag.as_ref().unwrap();
        assert_eq!(
            drag.paths.len(),
            2,
            "both rows are being dragged: {:?}",
            drag.paths
        );
        let target = drag.target.as_ref().unwrap();
        assert!(
            target
                .refusal
                .as_deref()
                .is_some_and(|why| why.contains("a.txt")),
            "refused before release: {:?}",
            target.refusal
        );

        h.app.finish_drag();
        assert!(root.join("x/a.txt").is_file() && root.join("y/a.txt").is_file());
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// A removal that stops partway must not ask about the paths that already
    /// went — the tree no longer shows those rows, so a report that mentioned
    /// only the failure would leave them unexplained.
    #[test]
    fn a_partial_trash_reports_what_went_and_what_did_not() {
        let root = scratch("partial-trash");
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(&root, &[("a.txt", NodeKind::File)]);
        // The second path does not exist, so the removal stops after the first.
        h.app
            .trash_now(vec![root.join("a.txt"), root.join("ghost.txt")]);

        assert!(!root.join("a.txt").exists(), "the first one went");
        assert!(
            h.app.status.contains("went") && h.app.status.contains("not deleted"),
            "it reports both halves: {:?}",
            h.app.status
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Hovering a collapsed directory opens it — except an ignored one, which
    /// stays shut while remaining a legal target (ADR 0032).
    #[test]
    fn a_drag_springs_a_directory_open_but_never_an_ignored_one() {
        let root = scratch("spring");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("a.txt"), b"x").unwrap();
        let mut h = rooted(
            &root,
            &[
                ("node_modules", NodeKind::Dir),
                ("sub", NodeKind::Dir),
                ("a.txt", NodeKind::File),
            ],
        );
        h.app.git_state = Some(Arc::new(parse_porcelain_v2(b"! node_modules/\0", &root)));
        let rows = h.app.rows();
        let row_of = |name: &str| rows.iter().position(|r| r.path == root.join(name)).unwrap();

        super::tests::press(&mut h.app, &rows, row_of("a.txt"), false);
        h.app.update_drag(pane(), &rows, 4, row_of("sub") as u16);
        assert!(
            h.app.tree.node_at(&root.join("sub")).unwrap().expanded,
            "a normal directory springs open"
        );

        h.app
            .update_drag(pane(), &rows, 4, row_of("node_modules") as u16);
        assert!(
            !h.app
                .tree
                .node_at(&root.join("node_modules"))
                .unwrap()
                .expanded,
            "an ignored one stays shut"
        );
        let target = h.app.drag.as_ref().unwrap().target.as_ref().unwrap();
        assert_eq!(
            target.directory,
            root.join("node_modules"),
            "but is still a target"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    /// `Esc` backs out one layer at a time, outermost first (ADR 0030).
    #[test]
    fn esc_backs_out_the_layers_in_order() {
        let mut h = harness(Mode::Tree);
        feed(&mut h.app, "/r", &[("a.txt", NodeKind::File)]);
        h.app.index = Some(super::tests::index_of(&[("a.txt", false)]));
        h.app.search_push('a');
        let rows = h.app.rows();
        assert!(
            h.app
                .open_menu(&rows, Some(PathBuf::from("/r/a.txt")), 2, 1)
        );
        h.app.edit = Some(Editor::create("/r".into(), false));

        h.app.handle_edit_input(InputAction::Esc);
        assert!(
            h.app.edit.is_none() && h.app.menu.is_some(),
            "then the edit"
        );
        assert_eq!(h.app.menu_input(pane(), InputAction::Esc), MenuStep::Close);
        h.app.menu = None;
        assert!(!h.app.on_esc(), "then the search, without quitting");
        assert!(h.app.search.is_none());
        assert!(h.app.on_esc(), "and only then, quit");
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
