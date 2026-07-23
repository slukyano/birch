//! Git state as a side-table snapshot (ADR 0005): a worker thread shells out
//! to `git status --porcelain=v2 -z` and parses the output into an immutable
//! `GitState` the view reads at flatten time. The tree knows nothing of git.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self, JoinHandle};

/// Per-file status, declared in badge-severity order: when a dir rolls up its
/// descendants, the smallest discriminant wins.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum FileStatus {
    Conflicted,
    Deleted,
    Renamed,
    Modified,
    Added,
    Untracked,
}

impl FileStatus {
    pub fn badge(self) -> char {
        match self {
            FileStatus::Conflicted => 'C',
            FileStatus::Deleted => 'D',
            FileStatus::Renamed => 'R',
            FileStatus::Modified => 'M',
            FileStatus::Added => 'A',
            FileStatus::Untracked => 'U',
        }
    }
}

/// Immutable snapshot of a repo's status, keyed by real (absolute) paths.
#[derive(Default, Debug)]
pub struct GitState {
    files: HashMap<PathBuf, FileStatus>,
    /// Rollup per dir: the most severe status among its descendants.
    dirs: HashMap<PathBuf, FileStatus>,
    /// Deleted-but-tracked file names grouped by parent dir.
    deleted: HashMap<PathBuf, Vec<String>>,
    /// Ignored files and dirs as reported (dirs collapse their subtrees).
    ignored: HashSet<PathBuf>,
    repo_root: PathBuf,
}

impl GitState {
    pub fn status_of(&self, path: &Path) -> Option<FileStatus> {
        self.files.get(path).copied()
    }

    /// The rollup for a dir — set even for collapsed or never-loaded dirs.
    pub fn dir_status(&self, path: &Path) -> Option<FileStatus> {
        self.dirs.get(path).copied()
    }

    /// Names of deleted-but-tracked files directly under `dir`.
    pub fn deleted_in(&self, dir: &Path) -> &[String] {
        self.deleted.get(dir).map(Vec::as_slice).unwrap_or(&[])
    }

    /// True when the path or any ancestor (up to the repo root) is ignored.
    pub fn is_ignored(&self, path: &Path) -> bool {
        let mut current = Some(path);
        while let Some(p) = current {
            if self.ignored.contains(p) {
                return true;
            }
            if p == self.repo_root {
                return false;
            }
            current = p.parent();
        }
        false
    }
}

/// Finds the enclosing repo by walking up from `start` looking for `.git`.
pub fn discover_repo(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

/// Parses `git status --porcelain=v2 -z` output. Paths in the output are
/// repo-root-relative; the returned state keys absolute paths.
pub fn parse_porcelain_v2(output: &[u8], repo_root: &Path) -> GitState {
    let mut state = GitState {
        repo_root: repo_root.to_path_buf(),
        ..GitState::default()
    };
    let mut records = output.split(|&b| b == 0).peekable();
    while let Some(record) = records.next() {
        let Ok(record) = std::str::from_utf8(record) else {
            continue;
        };
        if record.is_empty() {
            continue;
        }
        let (kind, rest) = match record.split_once(' ') {
            Some(split) => split,
            None => continue,
        };
        match kind {
            "1" => {
                // 1 <XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>
                let mut fields = rest.splitn(8, ' ');
                let xy = fields.next().unwrap_or("");
                let path = fields.nth(6).unwrap_or("");
                if let Some(status) = status_from_xy(xy) {
                    record_file(&mut state, repo_root.join(path), status);
                }
            }
            "2" => {
                // 2 <XY> ... <path> NUL <origPath>; the NUL split put origPath
                // in the next record — consume it.
                let mut fields = rest.splitn(9, ' ');
                let xy = fields.next().unwrap_or("");
                let path = fields.nth(7).unwrap_or("");
                records.next(); // discard origPath
                // A rename whose new path was then deleted in the worktree
                // (e.g. XY = RD) is a deletion for display purposes.
                let status = if xy.contains('D') {
                    FileStatus::Deleted
                } else {
                    FileStatus::Renamed
                };
                record_file(&mut state, repo_root.join(path), status);
            }
            "u" => {
                // u <XY> <sub> <m1> <m2> <m3> <mW> <h1> <h2> <h3> <path>
                let path = rest.splitn(10, ' ').nth(9).unwrap_or("");
                record_file(&mut state, repo_root.join(path), FileStatus::Conflicted);
            }
            "?" => record_file(&mut state, repo_root.join(rest), FileStatus::Untracked),
            "!" => {
                let path = rest.strip_suffix('/').unwrap_or(rest);
                state.ignored.insert(repo_root.join(path));
            }
            _ => {} // headers ("#") and unknown kinds
        }
    }
    state
}

fn status_from_xy(xy: &str) -> Option<FileStatus> {
    let has = |c: char| xy.contains(c);
    if has('U') {
        Some(FileStatus::Conflicted)
    } else if has('D') {
        Some(FileStatus::Deleted)
    } else if has('R') {
        Some(FileStatus::Renamed)
    } else if has('A') {
        Some(FileStatus::Added)
    } else if has('M') || has('T') {
        Some(FileStatus::Modified)
    } else {
        None
    }
}

fn record_file(state: &mut GitState, path: PathBuf, status: FileStatus) {
    if status == FileStatus::Deleted
        && let (Some(parent), Some(name)) = (path.parent(), path.file_name())
    {
        state
            .deleted
            .entry(parent.to_path_buf())
            .or_default()
            .push(name.to_string_lossy().into_owned());
    }
    // Roll the status up every ancestor dir inside the repo.
    let mut current = path.parent();
    while let Some(dir) = current {
        let entry = state.dirs.entry(dir.to_path_buf()).or_insert(status);
        *entry = (*entry).min(status);
        if dir == state.repo_root {
            break;
        }
        current = dir.parent();
    }
    state.files.insert(path, status);
}

#[derive(Clone, Debug)]
pub enum GitCmd {
    /// Refresh status for the given repo (it can change via `set-root`).
    Refresh { repo: PathBuf },
}

#[derive(Clone, Debug)]
pub enum GitEvent {
    /// A fresh snapshot (or None when status could not be read).
    State(Option<Arc<GitState>>),
}

pub struct GitWorker;

impl GitWorker {
    /// Spawns the worker. Consecutive queued `Refresh` commands collapse into
    /// one run (keeping the most recent repo).
    pub fn spawn(commands: Receiver<GitCmd>, events: Sender<GitEvent>) -> JoinHandle<()> {
        thread::Builder::new()
            .name("git-status".into())
            .spawn(move || {
                while let Ok(GitCmd::Refresh { mut repo }) = commands.recv() {
                    while let Ok(GitCmd::Refresh { repo: r }) = commands.try_recv() {
                        repo = r;
                    }
                    let state = run_status(&repo).map(Arc::new);
                    if events.send(GitEvent::State(state)).is_err() {
                        return;
                    }
                }
            })
            .expect("spawn git-status thread")
    }
}

fn run_status(repo_root: &Path) -> Option<GitState> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args([
            "status",
            "--porcelain=v2",
            "-z",
            // `matching` (not `traditional`): with --untracked-files=all,
            // traditional enumerates ignored dirs file-by-file and never
            // emits the `! dir/` record the ignored-set needs.
            "--ignored=matching",
            "--untracked-files=all",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_porcelain_v2(&output.stdout, repo_root))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(records: &[&str]) -> GitState {
        let joined = records.join("\0") + "\0";
        parse_porcelain_v2(joined.as_bytes(), Path::new("/repo"))
    }

    #[test]
    fn parses_all_record_kinds() {
        let state = parse(&[
            "1 .M N... 100644 100644 100644 abc def src/main.rs",
            "1 A. N... 000000 100644 100644 000 def src/new.rs",
            "1 .D N... 100644 100644 000000 abc abc src/gone.rs",
            "2 R. N... 100644 100644 100644 abc abc R100 src/to.rs",
            "src/from.rs",
            "u UU N... 100644 100644 100644 100644 a b c src/conflict.rs",
            "? notes.txt",
            "! target/",
        ]);
        assert_eq!(
            state.status_of(Path::new("/repo/src/main.rs")),
            Some(FileStatus::Modified)
        );
        assert_eq!(
            state.status_of(Path::new("/repo/src/new.rs")),
            Some(FileStatus::Added)
        );
        assert_eq!(
            state.status_of(Path::new("/repo/src/gone.rs")),
            Some(FileStatus::Deleted)
        );
        assert_eq!(
            state.status_of(Path::new("/repo/src/to.rs")),
            Some(FileStatus::Renamed)
        );
        assert_eq!(
            state.status_of(Path::new("/repo/src/conflict.rs")),
            Some(FileStatus::Conflicted)
        );
        assert_eq!(
            state.status_of(Path::new("/repo/notes.txt")),
            Some(FileStatus::Untracked)
        );
        assert!(state.is_ignored(Path::new("/repo/target")));
    }

    #[test]
    fn rollup_takes_most_severe_and_reaches_root() {
        let state = parse(&[
            "? a/b/new.txt",
            "1 .M N... 100644 100644 100644 x y a/b/mod.rs",
            "1 .D N... 100644 100644 000000 x x a/gone.rs",
        ]);
        assert_eq!(
            state.dir_status(Path::new("/repo/a/b")),
            Some(FileStatus::Modified)
        );
        assert_eq!(
            state.dir_status(Path::new("/repo/a")),
            Some(FileStatus::Deleted)
        );
        assert_eq!(
            state.dir_status(Path::new("/repo")),
            Some(FileStatus::Deleted)
        );
        assert_eq!(state.dir_status(Path::new("/repo/other")), None);
    }

    #[test]
    fn deleted_files_group_by_parent() {
        let state = parse(&[
            "1 .D N... 100644 100644 000000 x x src/a.rs",
            "1 D. N... 100644 000000 000000 x x src/b.rs",
        ]);
        let mut names = state.deleted_in(Path::new("/repo/src")).to_vec();
        names.sort();
        assert_eq!(names, ["a.rs", "b.rs"]);
        assert!(state.deleted_in(Path::new("/repo")).is_empty());
    }

    #[test]
    fn ignored_is_inherited_by_descendants() {
        let state = parse(&["! target/", "! debug.log"]);
        assert!(state.is_ignored(Path::new("/repo/target/deep/file.o")));
        assert!(state.is_ignored(Path::new("/repo/debug.log")));
        assert!(!state.is_ignored(Path::new("/repo/src/main.rs")));
    }

    #[test]
    fn staged_and_unstaged_xy_combinations() {
        assert_eq!(status_from_xy("MM"), Some(FileStatus::Modified));
        assert_eq!(status_from_xy("AM"), Some(FileStatus::Added));
        assert_eq!(status_from_xy("MD"), Some(FileStatus::Deleted));
        assert_eq!(status_from_xy(".T"), Some(FileStatus::Modified));
        assert_eq!(status_from_xy(".."), None);
    }

    #[test]
    fn rename_records_with_spaces_and_deleted_renames() {
        let state = parse(&[
            "2 R. N... 100644 100644 100644 abc abc R100 new dir/new name.rs",
            "old dir/old name.rs",
            "2 RD N... 100644 100644 000000 abc abc R100 was renamed.rs",
            "old.rs",
        ]);
        assert_eq!(
            state.status_of(Path::new("/repo/new dir/new name.rs")),
            Some(FileStatus::Renamed)
        );
        assert_eq!(
            state.status_of(Path::new("/repo/was renamed.rs")),
            Some(FileStatus::Deleted)
        );
        assert_eq!(state.deleted_in(Path::new("/repo")), ["was renamed.rs"]);
    }

    /// Runs the real `git status` invocation `run_status` uses, so the flag
    /// set is exercised against actual git output — hand-written fixtures
    /// cannot catch flags that change the record shapes.
    #[test]
    fn run_status_against_a_real_repo() {
        let git_works = Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success());
        assert!(git_works, "these tests require git on PATH");

        let tmp = std::env::temp_dir().join(format!("birch-git-e2e-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("src")).unwrap();
        let tmp = tmp.canonicalize().unwrap();
        let git = |args: &[&str]| {
            let ok = Command::new("git")
                .arg("-C")
                .arg(&tmp)
                .args(args)
                .output()
                .unwrap()
                .status
                .success();
            assert!(ok, "git {args:?} failed");
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "test@birch.invalid"]);
        git(&["config", "user.name", "birch test"]);
        std::fs::write(tmp.join(".gitignore"), "target/\n").unwrap();
        std::fs::write(tmp.join("src/main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(tmp.join("tracked.txt"), "keep\n").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-qm", "init"]);

        std::fs::write(tmp.join("src/main.rs"), "changed\n").unwrap(); // modified
        std::fs::write(tmp.join("untracked.txt"), "new\n").unwrap(); // untracked
        std::fs::remove_file(tmp.join("tracked.txt")).unwrap(); // deleted
        std::fs::create_dir_all(tmp.join("target/debug")).unwrap(); // ignored dir
        std::fs::write(tmp.join("target/debug/out.o"), "obj\n").unwrap();

        let state = run_status(&tmp).expect("git status runs");
        assert_eq!(
            state.status_of(&tmp.join("src/main.rs")),
            Some(FileStatus::Modified)
        );
        assert_eq!(
            state.status_of(&tmp.join("untracked.txt")),
            Some(FileStatus::Untracked)
        );
        assert_eq!(
            state.status_of(&tmp.join("tracked.txt")),
            Some(FileStatus::Deleted)
        );
        assert_eq!(state.deleted_in(&tmp), ["tracked.txt"]);
        // The ignored *dir* itself must be in the ignored set (the flag
        // combination is exactly what this asserts) and inherited below.
        assert!(state.is_ignored(&tmp.join("target")));
        assert!(state.is_ignored(&tmp.join("target/debug/out.o")));
        assert!(!state.is_ignored(&tmp.join("src/main.rs")));
        assert_eq!(
            state.dir_status(&tmp.join("src")),
            Some(FileStatus::Modified)
        );
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn discover_repo_walks_up() {
        let tmp = std::env::temp_dir().join(format!("birch-git-test-{}", std::process::id()));
        let nested = tmp.join("a/b");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir_all(tmp.join(".git")).unwrap();
        assert_eq!(discover_repo(&nested), Some(tmp.clone()));
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
