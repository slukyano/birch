//! The Files source: the default source, backed by the filesystem. Lazily
//! reads one directory level per `Expand` command.

use std::fs;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::source::{SourceCmd, SourceEvent};
use crate::tree::{Entry, NodeKind, TreeDelta};

pub struct FilesSource;

impl FilesSource {
    /// Spawns the worker thread. It exits when the command channel closes or
    /// the event channel is gone.
    pub fn spawn(commands: Receiver<SourceCmd>, events: Sender<SourceEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("files-source".into())
            .spawn(move || {
                while let Ok(cmd) = commands.recv() {
                    match cmd {
                        SourceCmd::Expand(dir) => {
                            let (entries, error) = read_entries(&dir);
                            if let Some(message) = error
                                && events.send(SourceEvent::Message(message)).is_err()
                            {
                                return;
                            }
                            let delta = TreeDelta::Snapshot { dir, entries };
                            if events.send(SourceEvent::Deltas(vec![delta])).is_err() {
                                return;
                            }
                        }
                    }
                }
            })
            .expect("spawn files-source thread")
    }
}

/// Reads one directory level. On failure returns no entries plus a message —
/// an unreadable dir renders as empty-but-loaded, never a crash.
fn read_entries(dir: &Path) -> (Vec<Entry>, Option<String>) {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => {
            return (
                Vec::new(),
                Some(format!("cannot read {}: {e}", dir.display())),
            );
        }
    };
    let mut entries = Vec::new();
    for dir_entry in read_dir.flatten() {
        let name = dir_entry.file_name().to_string_lossy().into_owned();
        let kind = match dir_entry.file_type() {
            Ok(ft) if ft.is_symlink() => match fs::metadata(dir_entry.path()) {
                Ok(target) if target.is_dir() => NodeKind::SymlinkDir,
                _ => NodeKind::SymlinkFile, // broken symlinks render as files
            },
            Ok(ft) if ft.is_dir() => NodeKind::Dir,
            _ => NodeKind::File,
        };
        entries.push(Entry { name, kind });
    }
    (entries, None)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;

    #[test]
    fn expand_emits_added_for_real_dir() {
        let tmp = std::env::temp_dir().join(format!("birch-fs-test-{}", std::process::id()));
        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("file.txt"), b"x").unwrap();

        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        let handle = FilesSource::spawn(cmd_rx, ev_tx);
        cmd_tx.send(SourceCmd::Expand(tmp.clone())).unwrap();
        drop(cmd_tx);

        let event = ev_rx.recv().unwrap();
        let SourceEvent::Deltas(deltas) = event else {
            panic!("expected deltas, got {event:?}");
        };
        let TreeDelta::Snapshot { dir, entries } = &deltas[0] else {
            panic!("expected Snapshot");
        };
        assert_eq!(dir, &tmp);
        let mut names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, ["file.txt", "sub"]);

        handle.join().unwrap();
        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn unreadable_dir_yields_message_and_empty_added() {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (ev_tx, ev_rx) = mpsc::channel();
        let handle = FilesSource::spawn(cmd_rx, ev_tx);
        cmd_tx
            .send(SourceCmd::Expand("/nonexistent/birch/test/dir".into()))
            .unwrap();
        drop(cmd_tx);

        let SourceEvent::Message(msg) = ev_rx.recv().unwrap() else {
            panic!("expected message first");
        };
        assert!(msg.contains("cannot read"));
        let SourceEvent::Deltas(deltas) = ev_rx.recv().unwrap() else {
            panic!("expected deltas");
        };
        let TreeDelta::Snapshot { entries, .. } = &deltas[0] else {
            panic!("expected Snapshot");
        };
        assert!(entries.is_empty());
        handle.join().unwrap();
    }
}
