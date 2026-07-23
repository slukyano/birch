//! The control-socket server (ADRs 0010/0011): a listener thread accepts
//! connections; each connection reads NDJSON requests and round-trips them
//! through the app loop over the unified event channel.

use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Sender, SyncSender, sync_channel};
use std::thread;

use birch_core::protocol::{self, Request, Response};

use crate::AppEvent;

/// A request in flight from a connection thread to the app loop.
pub struct CtlRequest {
    pub request: Request,
    pub reply: SyncSender<Response>,
}

/// What main needs for cleanup on exit.
#[derive(Debug)]
pub struct SocketHandle {
    pub socket_path: PathBuf,
    /// Present under default addressing (absent for `--socket`).
    pub by_root_link: Option<PathBuf>,
    dir: Option<PathBuf>,
}

impl SocketHandle {
    /// Re-points the by-root symlink after `set-root`, retiring the old
    /// root's link so stale clients get "no instance" instead of a tree
    /// rooted somewhere else.
    pub fn repoint(&mut self, new_root: &Path) {
        let Some(dir) = &self.dir else { return };
        self.remove_own_link();
        let link = protocol::by_root_link(dir, new_root);
        if link_to(&link, &self.socket_path).is_ok() {
            self.by_root_link = Some(link);
        } else {
            self.by_root_link = None;
        }
    }

    pub fn cleanup(&self) {
        let _ = fs::remove_file(&self.socket_path);
        self.remove_own_link();
    }

    /// Removes the by-root link only if it still points at this instance —
    /// a newer instance on the same root may have repointed it (most-recent-
    /// wins, ADR 0010) and must stay discoverable.
    fn remove_own_link(&self) {
        if let Some(link) = &self.by_root_link
            && fs::read_link(link).is_ok_and(|target| target == self.socket_path)
        {
            let _ = fs::remove_file(link);
        }
    }
}

/// Binds the socket (explicit path or default addressing) and spawns the
/// accept loop. The auth model is filesystem permissions, so the default dir
/// must actually be private — wrong ownership or mode is a hard error.
pub fn serve(
    explicit: Option<PathBuf>,
    root: &Path,
    events: Sender<AppEvent>,
) -> io::Result<SocketHandle> {
    let (socket_path, by_root, dir) = match explicit {
        Some(path) => (path, None, None),
        None => {
            let dir = protocol::socket_dir();
            ensure_private_dir(&dir)?;
            ensure_private_dir(&dir.join("by-root"))?;
            let socket = protocol::instance_socket(&dir, std::process::id());
            let link = protocol::by_root_link(&dir, root);
            (socket, Some(link), Some(dir))
        }
    };

    // A leftover socket from a crashed instance: unlink if nothing listens.
    // Only ever unlink an actual socket — an explicit --socket pointing at a
    // regular file must fail, not delete the file.
    if let Ok(meta) = fs::symlink_metadata(&socket_path) {
        if !meta.file_type().is_socket() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} exists and is not a socket", socket_path.display()),
            ));
        }
        match UnixStream::connect(&socket_path) {
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!("{} is in use by a live instance", socket_path.display()),
                ));
            }
            Err(_) => fs::remove_file(&socket_path)?,
        }
    }
    let listener = UnixListener::bind(&socket_path)?;
    let mut perms = fs::metadata(&socket_path)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(&socket_path, perms)?;

    if let Some(link) = &by_root {
        link_to(link, &socket_path)?;
    }

    thread::Builder::new()
        .name("ctl-accept".into())
        .spawn(move || {
            for stream in listener.incoming().flatten() {
                let events = events.clone();
                let _ = thread::Builder::new()
                    .name("ctl-conn".into())
                    .spawn(move || serve_connection(stream, events));
            }
        })
        .expect("spawn ctl-accept thread");

    Ok(SocketHandle {
        socket_path,
        by_root_link: by_root,
        dir,
    })
}

fn serve_connection(stream: UnixStream, events: Sender<AppEvent>) {
    let Ok(read) = stream.try_clone() else { return };
    let mut write = stream;
    // Cap what one connection may send: the auth model is same-uid, so this
    // guards memory against accidents, not attackers.
    let read = std::io::Read::take(read, 1 << 20);
    for line in BufReader::new(read).lines() {
        let Ok(line) = line else { return };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Request>(&line) {
            Err(e) => Response::err(format!("bad request: {e}")),
            Ok(request) => {
                let (reply_tx, reply_rx) = sync_channel(1);
                if events
                    .send(AppEvent::Ctl(CtlRequest {
                        request,
                        reply: reply_tx,
                    }))
                    .is_err()
                {
                    return; // app gone; drop the connection
                }
                match reply_rx.recv() {
                    Ok(response) => response,
                    Err(_) => return,
                }
            }
        };
        let Ok(mut line) = serde_json::to_string(&response) else {
            return;
        };
        line.push('\n');
        if write.write_all(line.as_bytes()).is_err() {
            return;
        }
    }
}

/// Filesystem permissions are the auth model (ADR 0010), so the dir must
/// actually be private: created 0700, and a pre-existing path is refused
/// unless it is a real dir (not a symlink) owned by us with no group/other
/// bits — never silently repaired, never followed through a planted link.
fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    match fs::symlink_metadata(dir) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let mut builder = fs::DirBuilder::new();
            std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
            builder.create(dir)?;
            // Validate what actually exists now (a raced creation included).
            let meta = fs::symlink_metadata(dir)?;
            validate_private(dir, &meta)
        }
        Err(e) => Err(e),
        Ok(meta) => validate_private(dir, &meta),
    }
}

fn validate_private(dir: &Path, meta: &fs::Metadata) -> io::Result<()> {
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(io::Error::other(format!(
            "socket dir {} is not a plain directory",
            dir.display()
        )));
    }
    if meta.uid() != protocol::effective_uid() {
        return Err(io::Error::other(format!(
            "socket dir {} is not owned by the current user",
            dir.display()
        )));
    }
    if meta.permissions().mode() & 0o077 != 0 {
        return Err(io::Error::other(format!(
            "socket dir {} is accessible to other users; expected mode 0700",
            dir.display()
        )));
    }
    Ok(())
}

fn link_to(link: &Path, target: &Path) -> io::Result<()> {
    let _ = fs::remove_file(link);
    std::os::unix::fs::symlink(target, link)
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use birch_core::protocol::Verb;

    use super::*;

    /// Stub app loop: answers every ctl request with an ok echoing the verb.
    fn stub_responder() -> Sender<AppEvent> {
        let (tx, rx) = mpsc::channel::<AppEvent>();
        thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if let AppEvent::Ctl(ctl) = event {
                    let verb = format!("{:?}", ctl.request.verb);
                    let _ = ctl.reply.send(Response::ok(Some(verb)));
                }
            }
        });
        tx
    }

    fn temp_socket(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("birch-ctl-test-{tag}-{}.sock", std::process::id()))
    }

    #[test]
    fn roundtrip_and_malformed_lines() {
        let path = temp_socket("rt");
        let _ = fs::remove_file(&path);
        let handle = serve(Some(path.clone()), Path::new("/r"), stub_responder()).unwrap();

        let mut stream = UnixStream::connect(&path).unwrap();
        stream
            .write_all(b"{\"v\":1,\"verb\":\"get-root\"}\nnot json\n{\"v\":1,\"verb\":\"quit\"}\n")
            .unwrap();
        let reader = BufReader::new(stream.try_clone().unwrap());
        let mut lines = reader.lines();

        let first: Response = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert!(first.ok);
        assert_eq!(first.data.as_deref(), Some("GetRoot"));
        // A malformed line answers with an error but keeps the connection.
        let second: Response = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert!(!second.ok);
        let third: Response = serde_json::from_str(&lines.next().unwrap().unwrap()).unwrap();
        assert!(third.ok);
        assert_eq!(third.data.as_deref(), Some("Quit"));

        handle.cleanup();
        assert!(!path.exists());
    }

    #[test]
    fn stale_socket_is_reclaimed_but_files_are_not() {
        let path = temp_socket("stale");
        let _ = fs::remove_file(&path);
        // A dead socket (bound, then dropped without unlink).
        let _ = UnixListener::bind(&path).unwrap();
        // (listener dropped here; the path remains as a dead socket file)
        let handle = serve(Some(path.clone()), Path::new("/r"), stub_responder())
            .expect("dead socket is reclaimed");
        handle.cleanup();

        // A regular file at the path must never be deleted.
        fs::write(&path, b"precious").unwrap();
        let err = serve(Some(path.clone()), Path::new("/r"), stub_responder())
            .expect_err("regular file refuses");
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read(&path).unwrap(), b"precious");
        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn request_shapes_cross_the_wire() {
        let path = temp_socket("wire");
        let _ = fs::remove_file(&path);
        let handle = serve(Some(path.clone()), Path::new("/r"), stub_responder()).unwrap();

        let mut request = Request::new(Verb::Reveal);
        request.path = Some("/r/some file with spaces.txt".into());
        let mut stream = UnixStream::connect(&path).unwrap();
        let mut line = serde_json::to_string(&request).unwrap();
        line.push('\n');
        stream.write_all(line.as_bytes()).unwrap();
        let mut reply = String::new();
        BufReader::new(stream).read_line(&mut reply).unwrap();
        let response: Response = serde_json::from_str(&reply).unwrap();
        assert!(response.ok);
        assert_eq!(response.data.as_deref(), Some("Reveal"));
        handle.cleanup();
    }
}
