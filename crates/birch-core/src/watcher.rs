//! Filesystem watching (ADR 0006): each expanded dir is watched
//! non-recursively; raw notify events are coalesced into debounced dirty-dir
//! batches, and a dirty dir simply gets re-scanned one level.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};

#[derive(Clone, Debug)]
pub enum WatchCmd {
    Watch(PathBuf),
    Unwatch(PathBuf),
}

#[derive(Clone, Debug)]
pub enum WatchEvent {
    /// Dirs whose one-level listing may have changed, coalesced over a quiet
    /// slice.
    Dirty(Vec<PathBuf>),
}

/// How long a batch accumulates before flushing.
pub const DEBOUNCE: Duration = Duration::from_millis(80);
/// A steady event stream still flushes at this cadence.
pub const MAX_LATENCY: Duration = Duration::from_millis(400);

/// The watch thread's single inbox: commands and raw notify paths merge into
/// one channel, so commands apply immediately instead of waiting out an idle
/// receive timeout.
enum Inbox {
    Cmd(WatchCmd),
    Raw(PathBuf),
}

pub struct FsWatcher;

impl FsWatcher {
    /// Spawns the watch thread. Watch/unwatch failures are silently ignored:
    /// a dir that vanished before the watch landed will produce no events,
    /// and the next snapshot of its parent removes it anyway.
    pub fn spawn(commands: Receiver<WatchCmd>, events: Sender<WatchEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("fs-watcher".into())
            .spawn(move || {
                let (inbox_tx, inbox_rx) = channel::<Inbox>();
                // Forward commands into the merged inbox.
                {
                    let inbox_tx = inbox_tx.clone();
                    thread::spawn(move || {
                        while let Ok(cmd) = commands.recv() {
                            if inbox_tx.send(Inbox::Cmd(cmd)).is_err() {
                                break;
                            }
                        }
                    });
                }
                let mut watcher = match RecommendedWatcher::new(
                    move |result: Result<notify::Event, notify::Error>| {
                        if let Ok(event) = result {
                            for path in event.paths {
                                let _ = inbox_tx.send(Inbox::Raw(path));
                            }
                        }
                    },
                    notify::Config::default(),
                ) {
                    Ok(w) => w,
                    Err(_) => return, // watching unavailable; live updates off
                };

                let mut debouncer = Debouncer::new(DEBOUNCE, MAX_LATENCY);
                loop {
                    match inbox_rx.recv_timeout(debouncer.wait()) {
                        Ok(Inbox::Cmd(WatchCmd::Watch(dir))) => {
                            if watcher.watch(&dir, RecursiveMode::NonRecursive).is_ok() {
                                // Changes between the expand-scan and the
                                // watch landing were unseen; one synthetic
                                // dirty re-scan self-heals that window.
                                if events.send(WatchEvent::Dirty(vec![dir])).is_err() {
                                    return;
                                }
                            }
                        }
                        Ok(Inbox::Cmd(WatchCmd::Unwatch(dir))) => {
                            let _ = watcher.unwatch(&dir);
                        }
                        Ok(Inbox::Raw(path)) => debouncer.record(path, Instant::now()),
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                    if let Some(batch) = debouncer.flush_if_due(Instant::now())
                        && events.send(WatchEvent::Dirty(batch)).is_err()
                    {
                        return;
                    }
                }
            })
            .expect("spawn fs-watcher thread")
    }
}

/// Pure debouncing state machine: records event paths (mapped to the dir
/// whose listing changed), flushes once no event arrived for a quiet slice —
/// or once the oldest pending event exceeds the max latency, so a steady
/// stream cannot defer flushing forever.
pub struct Debouncer {
    quiet: Duration,
    max_latency: Duration,
    pending: HashSet<PathBuf>,
    first_event: Option<Instant>,
    last_event: Option<Instant>,
}

impl Debouncer {
    pub fn new(quiet: Duration, max_latency: Duration) -> Self {
        Self {
            quiet,
            max_latency,
            pending: HashSet::new(),
            first_event: None,
            last_event: None,
        }
    }

    /// Records a raw event path. The dirty dir is the path's parent (create/
    /// remove/rename change the parent's listing; a content write maps to its
    /// dir too, which is harmless).
    pub fn record(&mut self, path: PathBuf, now: Instant) {
        if let Some(parent) = path.parent() {
            self.pending.insert(parent.to_path_buf());
        }
        self.first_event.get_or_insert(now);
        self.last_event = Some(now);
    }

    /// How long the event loop may sleep before a flush could be due.
    pub fn wait(&self) -> Duration {
        if self.pending.is_empty() {
            Duration::from_millis(500)
        } else {
            self.quiet
        }
    }

    /// Flushes when the quiet slice elapsed since the last event, or the
    /// max latency elapsed since the first pending one.
    pub fn flush_if_due(&mut self, now: Instant) -> Option<Vec<PathBuf>> {
        if self.pending.is_empty() {
            return None;
        }
        let quiet_due = self
            .last_event
            .is_some_and(|last| now.duration_since(last) >= self.quiet);
        let latency_due = self
            .first_event
            .is_some_and(|first| now.duration_since(first) >= self.max_latency);
        if !quiet_due && !latency_due {
            return None;
        }
        self.first_event = None;
        self.last_event = None;
        Some(self.pending.drain().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debouncer() -> Debouncer {
        Debouncer::new(Duration::from_millis(80), Duration::from_millis(400))
    }

    #[test]
    fn debouncer_coalesces_paths_to_parent_dirs() {
        let mut d = debouncer();
        let t0 = Instant::now();
        d.record("/r/src/a.rs".into(), t0);
        d.record("/r/src/b.rs".into(), t0);
        d.record("/r/readme.md".into(), t0);
        assert!(d.flush_if_due(t0).is_none()); // not quiet yet
        let mut batch = d
            .flush_if_due(t0 + Duration::from_millis(100))
            .expect("flush after quiet slice");
        batch.sort();
        assert_eq!(batch, [PathBuf::from("/r"), PathBuf::from("/r/src")]);
        assert!(d.flush_if_due(t0 + Duration::from_millis(200)).is_none());
    }

    #[test]
    fn new_events_extend_the_quiet_wait() {
        let mut d = debouncer();
        let t0 = Instant::now();
        d.record("/r/a".into(), t0);
        d.record("/r/b".into(), t0 + Duration::from_millis(60));
        // 100ms after t0 is only 40ms after the last event — still busy.
        assert!(d.flush_if_due(t0 + Duration::from_millis(100)).is_none());
        assert!(d.flush_if_due(t0 + Duration::from_millis(150)).is_some());
    }

    #[test]
    fn steady_stream_flushes_at_max_latency() {
        let mut d = debouncer();
        let t0 = Instant::now();
        // Events every 50ms — never a quiet 80ms slice.
        let mut t = t0;
        while t < t0 + Duration::from_millis(390) {
            d.record("/r/hot".into(), t);
            t += Duration::from_millis(50);
        }
        assert!(d.flush_if_due(t0 + Duration::from_millis(390)).is_none());
        d.record("/r/hot".into(), t0 + Duration::from_millis(410));
        assert!(
            d.flush_if_due(t0 + Duration::from_millis(410)).is_some(),
            "max latency must force a flush despite the steady stream"
        );
    }

    #[test]
    fn watcher_reports_dirty_dir_end_to_end() {
        let tmp = std::env::temp_dir().join(format!("birch-watch-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let tmp = tmp.canonicalize().unwrap();

        let (cmd_tx, cmd_rx) = channel();
        let (ev_tx, ev_rx) = channel();
        let _handle = FsWatcher::spawn(cmd_rx, ev_tx);
        cmd_tx.send(WatchCmd::Watch(tmp.clone())).unwrap();
        // The watch confirmation arrives as a synthetic dirty batch.
        let confirm = ev_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("synthetic dirty after watch lands");
        let WatchEvent::Dirty(dirs) = confirm;
        assert_eq!(dirs, std::slice::from_ref(&tmp));

        std::fs::write(tmp.join("new-file"), b"x").unwrap();
        let batch = ev_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("a dirty batch within five seconds");
        let WatchEvent::Dirty(dirs) = batch;
        assert!(
            dirs.iter().any(|d| d == &tmp),
            "expected {tmp:?} in {dirs:?}"
        );
        drop(cmd_tx);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
